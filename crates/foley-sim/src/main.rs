//! Plays Foley's beds through scripted scenes as the game would, without the
//! game: each bed is created, told its scene every few ticks as Foley's
//! controllers do, and read in the game's stream buffers (half a second each, as Foley returns them, by
//! default), all through the same C ABI the mod calls. Writes a WAV per bed
//! and reports what would be heard as wrong: dropouts, clicks, crackle, and
//! level steps in time with the reads.
//!
//! Also writes an SVG per bed (its level under its scene's params, reads and
//! dropouts marked) and `metrics.tsv`; `--baseline` compares against an
//! earlier run's.
//!
//! `cargo run --release -p foley-sim -- [scene|all] [--read SECONDS] [--out DIR] [--baseline FILE]`

mod analysis;
mod scenes;

use std::path::PathBuf;

use foley_ffi::{
    FOLEY_OK, FoleyBed, foley_bed_create, foley_bed_destroy, foley_bed_render, foley_bed_set_scene,
    foley_bed_strike,
};

const RATE: usize = 48_000;
/// The game's tick, samples.
const TICK: usize = RATE / 20;

fn main() {
    let mut which = String::from("all");
    let mut read_seconds = 0.5f32;
    let mut out = PathBuf::from("target/foley-sim");
    let mut baseline: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--read" => {
                read_seconds = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .expect("--read SECONDS")
            }
            "--out" => out = args.next().map(PathBuf::from).expect("--out DIR"),
            "--baseline" => baseline = args.next().map(PathBuf::from),
            "-h" | "--help" => {
                println!(
                    "foley-sim [scene|all] [--read SECONDS] [--out DIR] [--baseline FILE]\nscenes:"
                );
                for scene in scenes::all() {
                    println!("  {:16} {}", scene.name, scene.about);
                }
                return;
            }
            name => which = name.to_string(),
        }
    }
    std::fs::create_dir_all(&out).expect("make the output directory");
    let read = ((read_seconds * RATE as f32) as usize).max(TICK);
    println!(
        "Reads of {:.2} s, as the game's stream buffers; with its 4 queued, a scene change is heard up to {:.1} s late.\n",
        read as f32 / RATE as f32,
        4.0 * read as f32 / RATE as f32
    );
    let mut any = false;
    let mut failures = 0;
    let before: std::collections::HashMap<String, analysis::Metrics> = baseline
        .map(|path| std::fs::read_to_string(&path).expect("read the baseline"))
        .unwrap_or_default()
        .lines()
        .filter_map(analysis::Metrics::parse)
        .collect();
    let mut lines = Vec::new();
    for scene in scenes::all()
        .into_iter()
        .filter(|s| which == "all" || s.name == which)
    {
        any = true;
        println!("{} ({} s): {}", scene.name, scene.seconds, scene.about);
        for track in &scene.tracks {
            let samples = play(track, scene.seconds, read);
            let path = out.join(format!("{}-{}.wav", scene.name, track.name));
            analysis::write_wav(&path, &samples);
            let report = analysis::report(&samples, read);
            let name = format!("{}/{}", scene.name, track.name);
            let metrics = analysis::Metrics::of(&samples, read);
            lines.push(metrics.line(&name));
            let params: Vec<(f32, Vec<f32>)> = (0..=(scene.seconds * 4.0) as u32)
                .map(|k| (k as f32 / 4.0, (track.params)(k as f32 / 4.0)))
                .collect();
            let svg = analysis::plot(&samples, read, &params, &format!("{name}: {}", scene.about));
            std::fs::write(out.join(format!("{}-{}.svg", scene.name, track.name)), svg)
                .expect("write the plot");
            let mut problems = analysis::problems(&samples, read, smooth(track.kind));
            // A scene that names a fault the bed isn't otherwise checked for
            // (rain, fire, lava and thunder are drops and pops by nature): the
            // clip check stays, the named one is added.
            if let Some(expect) = track.expect {
                problems.extend(analysis::expected(&samples, read, expect));
            }
            println!(
                "  {:8} {} {:5.1} LUFS | {report}",
                track.name,
                if problems.is_empty() { "ok  " } else { "FAIL" },
                metrics.lufs
            );
            if let Some(was) = before.get(&name) {
                for change in metrics.changes(was) {
                    println!("           since the baseline: {change}");
                }
            }
            for problem in &problems {
                println!("           {problem}");
            }
            failures += problems.len();
            let gaps = analysis::dropout_times(&samples);
            if !gaps.is_empty() {
                println!("           dropouts at {:?} s", &gaps[..gaps.len().min(8)]);
            }
        }
        println!();
    }
    if which == "all" || which.starts_with("walks") {
        any = true;
        let seeds: u64 = which
            .strip_prefix("walks:")
            .and_then(|n| n.parse().ok())
            .unwrap_or(3);
        failures += walks(seeds, read, &out);
    }
    if which == "all" || which == "fuzz" {
        any = true;
        failures += fuzz();
    }
    if which == "all" || which == "stops" {
        any = true;
        failures += stops(read, &out);
    }
    if which == "all" || which == "mix" {
        any = true;
        failures += mix(read, &out);
    }
    assert!(any, "no scene called {which}; --help lists them");
    std::fs::write(out.join("metrics.tsv"), lines.join("\n") + "\n").expect("write the metrics");
    println!("WAVs in {}", out.display());
    if failures > 0 {
        eprintln!("{failures} check(s) failed");
        std::process::exit(1);
    }
}

/// The seam a bed makes when it is stopped and started again: a cave entered,
/// a portal, a world reloaded, a setting switched back on. Each bed is played
/// to its steady level, freed, and a fresh one given the same scene; the level
/// must not step across the join, and the fresh one must not fall silent. Wind
/// is the one bed the game stops without a fade (`SurroundBeds.stop` stops it
/// dead, where `PlacedBeds` fades water, fire and lava over 3 s), so this is
/// where that is heard.
fn stops(read: usize, out: &std::path::Path) -> usize {
    use foley_ffi::*;
    println!("stops: each bed stopped and started again, for a seam at the join");
    let scenes: [(&str, i32, Vec<f32>); 5] = [
        ("rain", FOLEY_BED_RAIN, {
            let mut p = vec![10.0];
            p.extend([0.0, 0.3, 0.2, 0.2, 0.3, 0.0, 0.0]);
            p.extend([-1.0, 0.0]);
            p
        }),
        ("wind", FOLEY_BED_WIND, vec![12.0, 1.0, 0.7]),
        ("water", FOLEY_BED_WATER, vec![20.0, 5.0, 1.0]),
        ("fire", FOLEY_BED_FIRE, vec![4.0, 1.0]),
        ("lava", FOLEY_BED_LAVA, vec![20.0, 1.0]),
    ];
    let mut failures = 0;
    for (name, kind, params) in &scenes {
        // Two seconds of the bed, then two seconds of a fresh one on the same
        // scene, joined: the join is where a stop and a start would be heard.
        let half = 2 * RATE;
        let mut pcm = vec![0i16; 2 * half];
        // SAFETY: each bed is written by create, freed before the next, and
        // every slice handed over is valid for its length.
        #[allow(unsafe_code)]
        unsafe {
            for (part, chunk) in pcm.chunks_mut(half).enumerate() {
                let mut bed: *mut FoleyBed = std::ptr::null_mut();
                assert_eq!(foley_bed_create(*kind, 11 + part as u64, &mut bed), FOLEY_OK);
                assert_eq!(
                    foley_bed_set_scene(bed, params.as_ptr(), params.len()),
                    FOLEY_OK,
                    "{name}"
                );
                let mut done = 0;
                while done < chunk.len() {
                    let n = read.min(chunk.len() - done);
                    assert_eq!(
                        foley_bed_render(bed, chunk[done..].as_mut_ptr(), n),
                        FOLEY_OK
                    );
                    done += n;
                }
                foley_bed_destroy(bed);
            }
        }
        analysis::write_wav(&out.join(format!("stop-{name}.wav")), &pcm);
        // The step from the last frame before the join to the first after it,
        // against the usual step within a frame: a seam reads as a jump.
        let x: Vec<f32> = pcm.iter().map(|s| f32::from(*s) / 32768.0).collect();
        let level = analysis::frame_levels(&x);
        let seam = level.len() / 2;
        let around = RATE / 2 / 480;
        let typical = {
            let mut v: Vec<f32> = (seam.saturating_sub(around)..seam)
                .chain((seam + 1)..(seam + around).min(level.len()))
                .map(|k| (level[k] - level[k - 1]).abs())
                .collect();
            v.sort_by(f32::total_cmp);
            if v.is_empty() {
                0.0
            } else {
                v[v.len() / 2]
            }
        };
        let step = (level[seam] - level[seam - 1]).abs();
        let before = level[seam - around..seam].iter().copied().fold(f32::MIN, f32::max);
        let after = level[seam..(seam + around).min(level.len())]
            .iter()
            .copied()
            .fold(f32::MIN, f32::max);
        // A seam is bad if the level jumps far more at the join than within a
        // frame of it, or if the fresh bed is far quieter than the old one.
        let jumped = step > (typical * 6.0).max(6.0);
        let fell = after < before - 12.0;
        let ok = !jumped && !fell;
        println!(
            "  {name:8} {} | step {step:5.1} dB (usual {typical:4.1}) | before {before:5.1} after {after:5.1} dB",
            if ok { "ok  " } else { "FAIL" }
        );
        if jumped {
            println!("           a {step:.1} dB jump at the join, against {typical:.1} dB within");
        }
        if fell {
            println!("           the fresh bed is {:.1} dB quieter than the one before", before - after);
        }
        failures += usize::from(!ok);
    }
    failures
}

/// A storm as the game plays it: several beds at once on one clock. The game
/// runs up to nine streams in an ordinary storm (rain and wind all round,
/// water, fire and lava where they are, thunder) and sums them in its mixer,
/// and no scene played two beds together, so the summed level went unmeasured
/// while a single rain bed peaks at -3 dBFS. The storm's beds are played
/// together here, each on its own scene schedule like its controller's: the
/// sum must keep headroom (nothing near full scale), and its loudness goes in
/// the metrics for a baseline.
fn mix(read: usize, out: &std::path::Path) -> usize {
    use foley_ffi::*;
    let storm = crate::scenes::all()
        .into_iter()
        .find(|s| s.name == "storm")
        .expect("the storm scene");
    println!(
        "mix: the storm's {} beds together, on one clock",
        storm.tracks.len()
    );
    struct Driver {
        bed: *mut FoleyBed,
        every: u32,
        params: fn(f32) -> Vec<f32>,
        pending: Option<Vec<f32>>,
        flashes: Vec<(f32, f32, f32, f32)>,
        strikes: std::iter::Peekable<std::slice::Iter<'static, (f32, f32, f32, f32)>>,
    }
    let mut drivers: Vec<Driver> = Vec::new();
    let seconds = storm.seconds;
    let total = (seconds * RATE as f32) as usize;
    let mut mixed = vec![0.0f32; total];
    // SAFETY: each bed is created here, rendered into this function's own
    // scratch, and freed before it returns; every slice handed over is valid
    // for its length.
    #[allow(unsafe_code)]
    unsafe {
        for track in storm.tracks {
            let mut bed: *mut FoleyBed = std::ptr::null_mut();
            assert_eq!(foley_bed_create(track.kind, 13, &mut bed), FOLEY_OK);
            drivers.push(Driver {
                bed,
                every: track.every,
                params: track.params,
                pending: Some((track.params)(0.0)),
                flashes: Vec::new(),
                strikes: track.strikes.iter().peekable(),
            });
        }
        let mut raw = vec![0i16; read];
        let mut acc = vec![0.0f32; read];
        let mut done = 0;
        let mut tick = 0u32;
        while done < total {
            while (tick as usize) * TICK <= done {
                let t = tick as f32 / 20.0;
                for d in &mut drivers {
                    if tick.is_multiple_of(d.every) {
                        d.pending = Some((d.params)(t));
                    }
                    while let Some((at, x, y, z)) = d.strikes.next_if(|s| s.0 <= t).copied() {
                        d.flashes.push((at, x, y, z));
                    }
                }
                tick += 1;
            }
            for d in &mut drivers {
                if let Some(p) = d.pending.take() {
                    assert_eq!(foley_bed_set_scene(d.bed, p.as_ptr(), p.len()), FOLEY_OK);
                }
                for (at, x, y, z) in d.flashes.drain(..) {
                    foley_bed_strike(d.bed, x, y, z, (at * 20.0) as u64);
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
            }
            let n = read.min(total - done);
            acc[..n].fill(0.0);
            for d in &mut drivers {
                assert_eq!(foley_bed_render(d.bed, raw.as_mut_ptr(), n), FOLEY_OK);
                for (sum, sample) in acc[..n].iter_mut().zip(&raw[..n]) {
                    *sum += f32::from(*sample) / 32768.0;
                }
            }
            mixed[done..done + n].copy_from_slice(&acc[..n]);
            done += n;
        }
        for d in &drivers {
            foley_bed_destroy(d.bed);
        }
    }
    let clipped = mixed.iter().filter(|s| **s >= 32_000.0 / 32768.0).count();
    let pcm: Vec<i16> = mixed.iter().map(|s| (s.clamp(-1.0, 0.9999) * 32768.0) as i16).collect();
    analysis::write_wav(&out.join("mix-storm.wav"), &pcm);
    let metrics = analysis::Metrics::of(&pcm, read);
    println!("  {:8} {} | peak {:5.1} dBFS | {:.1} LUFS",
        "storm",
        if clipped == 0 { "ok  " } else { "FAIL" },
        20.0 * (mixed.iter().map(|s| s.abs()).fold(1e-9f32, f32::max)).log10(),
        metrics.lufs,
    );
    if clipped > 0 {
        println!("           {clipped} samples past full scale in the sum");
    }
    usize::from(clipped > 0)
}

/// Whether a kind of bed should sound smooth (wind, water): no dropouts, no
/// crackle. Rain, fire, lava and thunder are drops, pops and rolls.
fn smooth(kind: i32) -> bool {
    kind == foley_ffi::FOLEY_BED_WIND || kind == foley_ffi::FOLEY_BED_WATER
}

/// Ten minutes of wandering per seed: wind and water as a player meets them,
/// each param easing to a new value somewhere in its usual range every 10 to
/// 40 s, sometimes snapping (a door, a teleport). The smooth beds must stay
/// smooth throughout.
fn walks(seeds: u64, read: usize, out: &std::path::Path) -> usize {
    use foley_ffi::{FOLEY_BED_WATER, FOLEY_BED_WIND};
    println!("walks: {seeds} seeds of ten minutes' wandering, wind and water");
    let mut failures = 0;
    for seed in 1..=seeds {
        let mut rng = Lcg(seed * 7919);
        for (name, kind, every, ranges) in [
            (
                "wind",
                FOLEY_BED_WIND,
                5,
                vec![(0.0, 20.0), (0.0, 1.0), (0.0, 1.0)],
            ),
            (
                "water",
                FOLEY_BED_WATER,
                10,
                vec![(0.0, 60.0), (0.0, 15.0), (0.0, 1.0)],
            ),
        ] {
            // Keyframes: (time, params, whether to snap there).
            let mut keys: Vec<(f32, Vec<f32>, bool)> = Vec::new();
            let mut t = 0.0;
            while t < 600.0 {
                let p = ranges
                    .iter()
                    .map(|(lo, hi)| lo + (hi - lo) * rng.unit().powi(2))
                    .collect();
                keys.push((t, p, rng.unit() < 0.15));
                t += 10.0 + 30.0 * rng.unit();
            }
            let at = move |t: f32| -> Vec<f32> {
                let k = keys.iter().rposition(|key| key.0 <= t).unwrap_or(0);
                let Some(next) = keys.get(k + 1) else {
                    return keys[k].1.clone();
                };
                if next.2 {
                    return keys[k].1.clone();
                }
                let x = ((t - keys[k].0) / (next.0 - keys[k].0)).clamp(0.0, 1.0);
                let x = x * x * (3.0 - 2.0 * x);
                keys[k]
                    .1
                    .iter()
                    .zip(&next.1)
                    .map(|(a, b)| a + (b - a) * x)
                    .collect()
            };
            let samples = play_with(name, kind, every, &at, &[], 600.0, read);
            let problems = analysis::problems(&samples, read, true);
            analysis::write_wav(&out.join(format!("walk{seed}-{name}.wav")), &samples);
            println!(
                "  seed {seed} {name:6} {} {}",
                if problems.is_empty() { "ok  " } else { "FAIL" },
                analysis::report(&samples, read)
            );
            for problem in &problems {
                println!("           {problem}");
            }
            failures += problems.len();
        }
    }
    failures
}

/// A tiny deterministic generator for the fuzzing.
struct Lcg(u64);

impl Lcg {
    fn unit(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Every bed, its scene thrown anywhere in range every tick and read in
/// buffers of any size, for a minute each: no call may fail, nothing may
/// clip past the level guard.
fn fuzz() -> usize {
    use foley_ffi::*;
    println!("fuzz: every bed, scenes anywhere in range every tick, reads of any size");
    let mut failures = 0;
    let mut rng = Lcg(42);
    for (name, kind) in [
        ("rain", FOLEY_BED_RAIN),
        ("wind", FOLEY_BED_WIND),
        ("water", FOLEY_BED_WATER),
        ("fire", FOLEY_BED_FIRE),
        ("lava", FOLEY_BED_LAVA),
        ("thunder", FOLEY_BED_THUNDER),
    ] {
        let total = 60 * RATE;
        let mut pcm = vec![0i16; total];
        let mut bed: *mut FoleyBed = std::ptr::null_mut();
        let mut errors = 0;
        // SAFETY: `bed` is written by create and freed last; slices are valid.
        #[allow(unsafe_code)]
        unsafe {
            assert_eq!(foley_bed_create(kind, 9, &mut bed), FOLEY_OK);
            let mut done = 0;
            while done < total {
                let r = &mut rng;
                let params: Vec<f32> = match kind {
                    FOLEY_BED_RAIN => {
                        let mut p = vec![200.0 * r.unit().powi(2)];
                        p.extend((0..FOLEY_SURFACES).map(|_| r.unit()));
                        p.push(if r.unit() < 0.5 {
                            -1.0
                        } else {
                            (r.unit() * 6.99).floor()
                        });
                        p.push(64.0 * r.unit());
                        p
                    }
                    FOLEY_BED_WIND => vec![60.0 * r.unit().powi(2), r.unit(), r.unit()],
                    FOLEY_BED_WATER => vec![
                        10_000.0 * r.unit().powi(4),
                        10_000.0 * r.unit().powi(4),
                        r.unit(),
                    ],
                    FOLEY_BED_FIRE | FOLEY_BED_LAVA => vec![10_000.0 * r.unit().powi(4), r.unit()],
                    _ => vec![-50.0 + 110.0 * r.unit(), 100.0 * r.unit(), r.unit()],
                };
                if foley_bed_set_scene(bed, params.as_ptr(), params.len()) != FOLEY_OK {
                    errors += 1;
                }
                if kind == FOLEY_BED_THUNDER && r.unit() < 0.02 {
                    foley_bed_strike(
                        bed,
                        2000.0 * r.unit() - 1000.0,
                        50.0 * r.unit(),
                        2000.0 * r.unit() - 1000.0,
                        1,
                    );
                }
                // A tick's worth on average, anywhere from one sample to a second.
                let n = ((RATE as f32 * r.unit().powi(3)) as usize).clamp(1, total - done);
                if foley_bed_render(bed, pcm[done..].as_mut_ptr(), n) != FOLEY_OK {
                    errors += 1;
                }
                done += n;
            }
            foley_bed_destroy(bed);
        }
        let clipped = pcm.iter().filter(|s| s.unsigned_abs() >= 32_000).count();
        let peak = pcm.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0);
        let ok = errors == 0 && clipped == 0;
        println!(
            "  {name:8} {} | failed calls {errors} | peak {:.1} dBFS | samples near full scale {clipped}",
            if ok { "ok  " } else { "FAIL" },
            20.0 * (f32::from(peak.max(1)) / 32768.0).log10()
        );
        failures += usize::from(!ok);
    }
    failures
}

/// Plays `track` for `seconds`: its scene set every `track.every` ticks, read
/// `read` samples at a time, as the game's stream thread reads it.
fn play(track: &scenes::Track, seconds: f32, read: usize) -> Vec<i16> {
    play_with(
        track.name,
        track.kind,
        track.every,
        &track.params,
        track.strikes,
        seconds,
        read,
    )
}

/// [`play`], from parts: params from any function of time.
fn play_with(
    name: &str,
    kind: i32,
    every: u32,
    params: &dyn Fn(f32) -> Vec<f32>,
    strikes: &[(f32, f32, f32, f32)],
    seconds: f32,
    read: usize,
) -> Vec<i16> {
    let total = (seconds * RATE as f32) as usize;
    let mut bed: *mut FoleyBed = std::ptr::null_mut();
    let mut pcm = vec![0i16; total];
    // SAFETY: `bed` is written by create and freed last; every slice handed
    // over is valid for its length.
    #[allow(unsafe_code)]
    unsafe {
        assert_eq!(foley_bed_create(kind, 7, &mut bed), FOLEY_OK);
        // What the game thread has handed over and the stream thread hasn't
        // taken up yet (BedSound's pending scene and queued flashes).
        let mut pending: Option<Vec<f32>> = Some(params(0.0));
        let mut flashes: Vec<(f32, f32, f32, f32)> = Vec::new();
        let mut strikes = strikes.iter().peekable();
        let mut done = 0;
        let mut tick = 0u32;
        while done < total {
            // The game ticks up to where this read starts.
            while (tick as usize) * TICK <= done {
                let t = tick as f32 / 20.0;
                if tick.is_multiple_of(every) {
                    pending = Some(params(t));
                }
                while let Some((at, x, y, z)) = strikes.next_if(|s| s.0 <= t).copied() {
                    flashes.push((at, x, y, z));
                }
                tick += 1;
            }
            // The stream thread takes up the latest, then reads.
            if let Some(p) = pending.take() {
                assert_eq!(
                    foley_bed_set_scene(bed, p.as_ptr(), p.len()),
                    FOLEY_OK,
                    "{} params",
                    name
                );
            }
            for (at, x, y, z) in flashes.drain(..) {
                let _ = at;
                foley_bed_strike(bed, x, y, z, (at * 20.0) as u64);
                // Thunder is made on its own thread, in real time; the game
                // waits seconds for it anyway, so here it's given the time.
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            let n = read.min(total - done);
            assert_eq!(foley_bed_render(bed, pcm[done..].as_mut_ptr(), n), FOLEY_OK);
            done += n;
        }
        foley_bed_destroy(bed);
    }
    pcm
}
