//! `cargo xtask levels`: how loud is each bed, and does any of them break the
//! level guard's 0.4 full-scale ceiling on its own?
//!
//! Every bed kind is created through the real C ABI, given a
//! `NativeBridge`-equivalent scene (the same params the mod's controllers
//! send), and rendered for 30 s at 48 kHz in the game's 128-frame blocks.
//! The table is sorted by peak; the command exits non-zero when a bed's own
//! output passes [`BED_CEILING`], so it can stand as a regression gate.
//!
//! The last block sums the four coastline beds (surf + wind + water + rain)
//! at their loudest measured scenes, with no guard on the sum, to show the
//! headroom the game's mixer has before full scale.
#![allow(unsafe_code)]

use std::time::Duration;

use anyhow::{Result, bail, ensure};
use foley_ffi::{
    FOLEY_BED_BIOME, FOLEY_BED_CAVE, FOLEY_BED_FIRE, FOLEY_BED_LAVA, FOLEY_BED_RAIN,
    FOLEY_BED_SURF, FOLEY_BED_THUNDER, FOLEY_BED_UNDERWATER, FOLEY_BED_WATER, FOLEY_BED_WIND,
    FOLEY_OK, FOLEY_SURFACES, FoleyBed, foley_bed_create, foley_bed_destroy, foley_bed_render,
    foley_bed_set_scene, foley_bed_strike,
};

/// The game's rate and stream block.
const RATE: usize = 48_000;
const BLOCK: usize = 128;
/// Long enough for the guard's envelope to settle, and for a thunder bed to
/// be struck more than once.
const SECONDS: usize = 30;
/// One fixed seed, as in the sim: a number that moves is a code change.
const SEED: u64 = 7;
/// The level guard's contract (`crates/foley-ffi/src/lib.rs`): a bed's own
/// output never goes past this peak. Kept here rather than exported, so this
/// harness asserts the same number the guard does.
const BED_CEILING: f32 = 0.4;
/// `to_pcm` scales the soft-limited sample by `i16::MAX`.
const PCM_SCALE: f32 = i16::MAX as f32;

const BIOME_KINDS: [&str; 7] = [
    "desert",
    "mushroom",
    "lush",
    "dripstone",
    "deep dark",
    "snow",
    "mangrove",
];

/// A flash's time and place: seconds into the render, metres from the
/// listener (y up), and a fixed seed.
struct Strike {
    at: f32,
    at_m: [f32; 3],
    seed: u64,
}

/// Strikes early enough after a long silence that the guard has risen, as it
/// does between real flashes: the worst case for a sudden onset.
const THUNDER_STRIKES: &[Strike] = &[
    Strike {
        at: 2.0,
        at_m: [50.0, 0.0, 0.0],
        seed: 1,
    },
    Strike {
        at: 12.0,
        at_m: [120.0, 0.0, 80.0],
        seed: 2,
    },
    Strike {
        at: 22.0,
        at_m: [900.0, 0.0, 0.0],
        seed: 3,
    },
];

struct Scenario {
    bed: &'static str,
    about: String,
    kind: i32,
    params: Vec<f32>,
    strikes: &'static [Strike],
}

#[derive(Clone, Copy)]
struct Stats {
    peak: f32,
    /// When the peak landed, seconds into the render.
    peak_at: f32,
    rms: f32,
    /// Percent of samples at or above the ceiling.
    at_ceiling: f64,
}

pub fn run() -> Result<()> {
    let scenarios = scenarios();
    // The same completeness check `every_bed_renders_pcm` makes: a new bed
    // kind must not slip past the sweep.
    for kind in 0..=FOLEY_BED_SURF {
        ensure!(
            scenarios.iter().any(|scenario| scenario.kind == kind),
            "no scenario for bed kind {kind}"
        );
    }

    let frames = SECONDS * RATE;
    println!("Foley bed levels: {SECONDS} s at {RATE} Hz in {BLOCK}-frame blocks, seed {SEED}");
    println!("Guard contract: a bed's own output stays at or below {BED_CEILING} full scale.");
    println!("Peak and RMS are recovered from the 16-bit PCM through to_pcm's tanh");
    println!("(x = atanh(pcm / 32767)): the guard's own domain. The PCM heard is soft-limited.\n");

    let mut results = Vec::with_capacity(scenarios.len());
    for scenario in &scenarios {
        results.push((scenario, measure(scenario, frames)?));
    }
    results.sort_by(|a, b| {
        b.1.peak
            .total_cmp(&a.1.peak)
            .then_with(|| a.0.bed.cmp(b.0.bed))
            .then_with(|| a.0.about.cmp(&b.0.about))
    });

    let params: Vec<String> = results
        .iter()
        .map(|(scenario, _)| params_text(&scenario.params))
        .collect();
    let param_width = params.iter().map(String::len).max().unwrap_or(6).max(6);
    let about_width = results
        .iter()
        .map(|(scenario, _)| scenario.about.len())
        .max()
        .unwrap_or(5)
        .max(5);

    println!(
        "{:<10} {:<about_width$} {:<param_width$} {:>7} {:>7} {:>7}",
        "bed", "scene", "params", "peak", "rms", ">=0.4"
    );
    println!(
        "{}",
        "-".repeat(10 + 1 + about_width + 1 + param_width + 1 + 7 + 1 + 7 + 1 + 7)
    );
    for ((scenario, stats), params) in results.iter().zip(&params) {
        println!(
            "{:<10} {:<about_width$} {:<param_width$} {:>7.4} {:>7.4} {:>6.3}%",
            scenario.bed, scenario.about, params, stats.peak, stats.rms, stats.at_ceiling,
        );
    }

    let over: Vec<_> = results
        .iter()
        .filter(|(_, stats)| stats.peak > BED_CEILING)
        .collect();
    println!();
    if over.is_empty() {
        println!("OK: no bed scene broke the {BED_CEILING} full-scale ceiling.");
    } else {
        println!(
            "{} bed scene(s) BROKE the {BED_CEILING} full-scale ceiling:",
            over.len()
        );
        for (scenario, stats) in &over {
            println!(
                "  {:<9} {:<about_width$} peak {:.4} at {:.3} s",
                scenario.bed, scenario.about, stats.peak, stats.peak_at
            );
        }
    }

    coastline(&results, frames)?;

    if !over.is_empty() {
        let offenders = over
            .iter()
            .map(|(scenario, _)| format!("{} ({})", scenario.bed, scenario.about))
            .collect::<Vec<_>>()
            .join(", ");
        bail!(
            "{} bed scene(s) exceeded the {BED_CEILING} full-scale ceiling: {offenders}. \
             This is a measurement tool, not a build blocker — the non-zero exit \
             exists only to flag scenes worth listening to.",
            over.len()
        );
    }
    Ok(())
}

#[allow(
    clippy::vec_init_then_push,
    reason = "one grouped push per bed reads better than a single flat vec![..]"
)]
fn scenarios() -> Vec<Scenario> {
    let mut out = Vec::new();

    // Rain: [rate, water, stone, leaves, wood, soft, glass, metal, roof,
    // height].
    out.push(scene(
        "rain",
        "rate 1 mm/h on water",
        FOLEY_BED_RAIN,
        rain(1.0, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ));
    out.push(scene(
        "rain",
        "rate 50 mm/h, the game's loudest",
        FOLEY_BED_RAIN,
        rain(50.0, [0.3, 0.2, 0.2, 0.0, 0.3, 0.0, 0.0]),
    ));
    out.push(scene(
        "rain",
        "rate 200 mm/h on all seven surfaces",
        FOLEY_BED_RAIN,
        rain(200.0, [1.0; FOLEY_SURFACES]),
    ));

    // Wind: [speed m/s, exposure, leaves].
    out.push(scene(
        "wind",
        "12 m/s, exposure 1, leaves 0.5",
        FOLEY_BED_WIND,
        vec![12.0, 1.0, 0.5],
    ));
    out.push(scene(
        "wind",
        "60 m/s, exposure 1, leaves 1",
        FOLEY_BED_WIND,
        vec![60.0, 1.0, 1.0],
    ));

    // Water: [flow blocks, falls blocks, gain].
    out.push(scene(
        "water",
        "flow 1, falls 1, gain 1",
        FOLEY_BED_WATER,
        vec![1.0, 1.0, 1.0],
    ));
    out.push(scene(
        "water",
        "flow 5, falls 5, gain 1",
        FOLEY_BED_WATER,
        vec![5.0, 5.0, 1.0],
    ));
    out.push(scene(
        "water",
        "flow 300, falls 20, gain 1 (the game's loudest)",
        FOLEY_BED_WATER,
        vec![300.0, 20.0, 1.0],
    ));

    // Fire: [blocks, gain].
    out.push(scene(
        "fire",
        "4 blocks, gain 1",
        FOLEY_BED_FIRE,
        vec![4.0, 1.0],
    ));
    out.push(scene(
        "fire",
        "200 blocks, gain 1",
        FOLEY_BED_FIRE,
        vec![200.0, 1.0],
    ));

    // Lava: [blocks, gain].
    out.push(scene(
        "lava",
        "20 blocks, gain 1",
        FOLEY_BED_LAVA,
        vec![20.0, 1.0],
    ));
    out.push(scene(
        "lava",
        "500 blocks, gain 1",
        FOLEY_BED_LAVA,
        vec![500.0, 1.0],
    ));

    // Thunder: [celsius, humidity %, exposure]. Two flashes land while the
    // bed is otherwise silent; the first is close, after 2 s.
    out.push(Scenario {
        bed: "thunder",
        about: "15 C, 90%, exposure 1, 50 m flash after 2 s".into(),
        kind: FOLEY_BED_THUNDER,
        params: vec![15.0, 90.0, 1.0],
        strikes: THUNDER_STRIKES,
    });
    out.push(Scenario {
        bed: "thunder",
        about: "40 C, 100%, exposure 1, 50 m flash after 2 s".into(),
        kind: FOLEY_BED_THUNDER,
        params: vec![40.0, 100.0, 1.0],
        strikes: THUNDER_STRIKES,
    });

    // Cave: [depth, mass, openness, drips, gain].
    out.push(scene(
        "cave",
        "deep, mass 1, open 0, drips 1, gain 1",
        FOLEY_BED_CAVE,
        vec![1.0, 1.0, 0.0, 1.0, 1.0],
    ));
    out.push(scene(
        "cave",
        "deep, mass 0.5, open 0.2, drips 0.5, gain 1",
        FOLEY_BED_CAVE,
        vec![1.0, 0.5, 0.2, 0.5, 1.0],
    ));

    // Underwater: [level].
    out.push(scene(
        "underwater",
        "level 1 (head fully under)",
        FOLEY_BED_UNDERWATER,
        vec![1.0],
    ));

    // Biome: [kind, intensity, sub, gain]; all seven kinds at full intensity
    // and the largest sub their scene takes.
    for (kind, name) in BIOME_KINDS.iter().enumerate() {
        out.push(Scenario {
            bed: "biome",
            about: format!("{name}, intensity 1, sub 200, gain 1"),
            kind: FOLEY_BED_BIOME,
            params: vec![kind as f32, 1.0, 200.0, 1.0],
            strikes: &[],
        });
    }

    // Surf: [height m, shore, storm, exposure].
    out.push(scene(
        "surf",
        "1 m, shore 1, storm 0.5, exposure 0.5",
        FOLEY_BED_SURF,
        vec![1.0, 1.0, 0.5, 0.5],
    ));
    out.push(scene(
        "surf",
        "3 m, shore 1, storm 1, exposure 1",
        FOLEY_BED_SURF,
        vec![3.0, 1.0, 1.0, 1.0],
    ));
    out.push(scene(
        "surf",
        "3 m offshore, storm 1, exposure 1",
        FOLEY_BED_SURF,
        vec![3.0, 0.0, 1.0, 1.0],
    ));

    out
}

fn scene(bed: &'static str, about: &str, kind: i32, params: Vec<f32>) -> Scenario {
    Scenario {
        bed,
        about: about.to_owned(),
        kind,
        params,
        strikes: &[],
    }
}

/// Rain's params: `rate` mm/h with a share per surface in Foley's order
/// (water, stone, leaves, wood, soft, glass, metal), no roof.
fn rain(rate: f32, shares: [f32; FOLEY_SURFACES]) -> Vec<f32> {
    let mut params = vec![0.0; 3 + FOLEY_SURFACES];
    params[0] = rate;
    params[1..=FOLEY_SURFACES].copy_from_slice(&shares);
    params[1 + FOLEY_SURFACES] = -1.0;
    params
}

fn params_text(params: &[f32]) -> String {
    params
        .iter()
        .map(|&x| {
            if x.fract() == 0.0 && x.abs() < 1e6 {
                format!("{}", x as i64)
            } else {
                format!("{x:.2}")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn measure(scenario: &Scenario, frames: usize) -> Result<Stats> {
    let mut bed = Bed::new(scenario.kind)?;
    bed.set(&scenario.params)?;
    let mut pcm = vec![0i16; BLOCK];
    let mut peak = 0.0f32;
    let mut peak_at = 0usize;
    let mut sum_sq = 0.0f64;
    let mut at_ceiling = 0u64;
    let mut next_strike = 0;
    let mut done = 0;
    while done < frames {
        while let Some(strike) = scenario.strikes.get(next_strike) {
            if (strike.at * RATE as f32) as usize > done {
                break;
            }
            bed.strike(strike.at_m, strike.seed)?;
            // The thunder is made on the bed's own thread and joins the mix
            // when it is ready; wait, as the game's next tick would.
            std::thread::sleep(Duration::from_millis(200));
            next_strike += 1;
        }
        bed.render(&mut pcm)?;
        for (i, &sample) in pcm.iter().enumerate() {
            let level = level(sample);
            if level.abs() > peak {
                peak = level.abs();
                peak_at = done + i;
            }
            sum_sq += f64::from(level * level);
            if level.abs() >= BED_CEILING {
                at_ceiling += 1;
            }
        }
        done += BLOCK;
    }
    Ok(Stats {
        peak,
        peak_at: peak_at as f32 / RATE as f32,
        rms: (sum_sq / frames as f64).sqrt() as f32,
        at_ceiling: 100.0 * at_ceiling as f64 / frames as f64,
    })
}

/// The level the guard saw: `to_pcm` soft-limits with tanh and scales by
/// `i16::MAX`, so undo both. Truncation to i16 only ever lowers the
/// magnitude, so the recovered level is a conservative (never high) estimate.
fn level(sample: i16) -> f32 {
    let y = (f32::from(sample) / PCM_SCALE).clamp(-0.999_999, 0.999_999);
    y.atanh()
}

fn loudest<'a>(results: &[(&'a Scenario, Stats)], bed: &str) -> Option<&'a Scenario> {
    results
        .iter()
        .filter(|(scenario, _)| scenario.bed == bed)
        .max_by(|a, b| a.1.peak.total_cmp(&b.1.peak))
        .map(|(scenario, _)| *scenario)
}

fn coastline(results: &[(&Scenario, Stats)], frames: usize) -> Result<()> {
    const COAST: [&str; 4] = ["rain", "wind", "water", "surf"];
    let chosen: Vec<&Scenario> = COAST
        .iter()
        .filter_map(|bed| loudest(results, bed))
        .collect();
    ensure!(chosen.len() == COAST.len(), "the coast needs all four beds");

    println!("\nLoudest simultaneous coastline set (each bed's own guard still on):");
    for scenario in &chosen {
        println!("  {:<6} {}", scenario.bed, scenario.about);
    }
    let (peak, rms) = measure_sum(&chosen, frames)?;
    let dbfs = 20.0 * peak.max(1e-9).log10();
    println!("  unguarded f32 sum of the PCM: peak {peak:.4}, rms {rms:.4} ({dbfs:.2} dBFS)");
    if peak > 1.0 {
        println!(
            "  the mixer clips: {:.2} dB over full scale, {:.4} above 1.0",
            20.0 * peak.log10(),
            peak - 1.0
        );
    } else {
        println!("  {:.2} dB of headroom before full scale", -dbfs);
    }
    Ok(())
}

fn measure_sum(scenarios: &[&Scenario], frames: usize) -> Result<(f32, f32)> {
    let mut beds = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        let bed = Bed::new(scenario.kind)?;
        bed.set(&scenario.params)?;
        beds.push(bed);
    }
    let mut sum = [0.0f32; BLOCK];
    let mut peak = 0.0f32;
    let mut sum_sq = 0.0f64;
    let mut done = 0;
    while done < frames {
        sum.fill(0.0);
        for bed in &mut beds {
            let mut pcm = [0i16; BLOCK];
            bed.render(&mut pcm)?;
            for (total, sample) in sum.iter_mut().zip(pcm) {
                *total += f32::from(sample) / 32768.0;
            }
        }
        for sample in sum {
            peak = peak.max(sample.abs());
            sum_sq += f64::from(sample * sample);
        }
        done += BLOCK;
    }
    Ok((peak, (sum_sq / frames as f64).sqrt() as f32))
}

/// A live `FoleyBed`, freed on drop even when a render fails.
struct Bed(*mut FoleyBed);

impl Bed {
    fn new(kind: i32) -> Result<Self> {
        let mut bed = std::ptr::null_mut();
        // SAFETY: `bed` points at a writable slot; the handle written to it
        // is owned by the returned `Bed`.
        let code = unsafe { foley_bed_create(kind, SEED, &mut bed) };
        ensure!(code == FOLEY_OK, "foley_bed_create({kind}) failed: {code}");
        ensure!(!bed.is_null(), "foley_bed_create({kind}) returned null");
        Ok(Bed(bed))
    }

    fn set(&self, params: &[f32]) -> Result<()> {
        // SAFETY: `self.0` is live until `Drop`, `params` is valid for its
        // length, and this harness uses the bed on one thread at a time.
        let code = unsafe { foley_bed_set_scene(self.0, params.as_ptr(), params.len()) };
        ensure!(
            code == FOLEY_OK,
            "foley_bed_set_scene({params:?}) failed: {code}"
        );
        Ok(())
    }

    fn strike(&self, at_m: [f32; 3], seed: u64) -> Result<()> {
        // SAFETY: `self.0` is live until `Drop`, and this harness uses the
        // bed on one thread at a time.
        let code = unsafe { foley_bed_strike(self.0, at_m[0], at_m[1], at_m[2], seed) };
        ensure!(
            code == FOLEY_OK,
            "foley_bed_strike({at_m:?}) failed: {code}"
        );
        Ok(())
    }

    fn render(&mut self, out: &mut [i16]) -> Result<()> {
        // SAFETY: `self.0` is live until `Drop`, and `out` is writable for
        // its length; this harness uses the bed on one thread at a time.
        let code = unsafe { foley_bed_render(self.0, out.as_mut_ptr(), out.len()) };
        ensure!(
            code == FOLEY_OK,
            "foley_bed_render({}) failed: {code}",
            out.len()
        );
        Ok(())
    }
}

impl Drop for Bed {
    fn drop(&mut self) {
        // SAFETY: `self.0` came from `foley_bed_create` and is not used again.
        unsafe { foley_bed_destroy(self.0) };
    }
}
