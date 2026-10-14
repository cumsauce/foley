//! Plays Foley's beds through scripted scenes as the game would, without the
//! game: each bed is created, told its scene every few ticks as Foley's
//! controllers do, and read in the game's stream buffers (1 s each, by
//! default), all through the same C ABI the mod calls. Writes a WAV per bed
//! and reports what would be heard as wrong: dropouts, clicks, crackle, and
//! level steps in time with the reads.
//!
//! `cargo run --release -p foley-sim -- [scene|all] [--read SECONDS] [--out DIR]`

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
    let mut read_seconds = 1.0f32;
    let mut out = PathBuf::from("target/foley-sim");
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
            "-h" | "--help" => {
                println!("foley-sim [scene|all] [--read SECONDS] [--out DIR]\nscenes:");
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
            println!("  {:8} {report}", track.name);
            let gaps = analysis::dropout_times(&samples);
            if !gaps.is_empty() {
                println!("           dropouts at {:?} s", &gaps[..gaps.len().min(8)]);
            }
        }
        println!();
    }
    assert!(any, "no scene called {which}; --help lists them");
    println!("WAVs in {}", out.display());
}

/// Plays `track` for `seconds`: its scene set every `track.every` ticks, read
/// `read` samples at a time, as the game's stream thread reads it.
fn play(track: &scenes::Track, seconds: f32, read: usize) -> Vec<i16> {
    let total = (seconds * RATE as f32) as usize;
    let mut bed: *mut FoleyBed = std::ptr::null_mut();
    let mut pcm = vec![0i16; total];
    // SAFETY: `bed` is written by create and freed last; every slice handed
    // over is valid for its length.
    #[allow(unsafe_code)]
    unsafe {
        assert_eq!(foley_bed_create(track.kind, 7, &mut bed), FOLEY_OK);
        // What the game thread has handed over and the stream thread hasn't
        // taken up yet (BedSound's pending scene and queued flashes).
        let mut pending: Option<Vec<f32>> = Some((track.params)(0.0));
        let mut flashes: Vec<f32> = Vec::new();
        let mut strikes = track.strikes.iter().peekable();
        let mut done = 0;
        let mut tick = 0u32;
        while done < total {
            // The game ticks up to where this read starts.
            while (tick as usize) * TICK <= done {
                let t = tick as f32 / 20.0;
                if tick.is_multiple_of(track.every) {
                    pending = Some((track.params)(t));
                }
                while strikes.next_if(|s| **s <= t).is_some() {
                    flashes.push(t);
                }
                tick += 1;
            }
            // The stream thread takes up the latest, then reads.
            if let Some(p) = pending.take() {
                assert_eq!(
                    foley_bed_set_scene(bed, p.as_ptr(), p.len()),
                    FOLEY_OK,
                    "{} params",
                    track.name
                );
            }
            for (n, _) in flashes.drain(..).enumerate() {
                let angle = n as f32 * 2.1;
                foley_bed_strike(bed, 900.0 * angle.cos(), 0.0, 900.0 * angle.sin(), n as u64);
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
