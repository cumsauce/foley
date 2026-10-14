//! What's wrong with a bed's sound, as numbers: dropouts, clicks, crackle,
//! and steps in its level in time with the game's reads.

use std::io::Write;
use std::path::Path;

const RATE: usize = 48_000;
/// Level frames: 10 ms.
const FRAME: usize = RATE / 100;

pub fn write_wav(path: &Path, samples: &[i16]) {
    let bytes = (samples.len() * 2) as u32;
    let mut file = std::io::BufWriter::new(std::fs::File::create(path).expect("create the WAV"));
    let mut header = Vec::with_capacity(44);
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&(36 + bytes).to_le_bytes());
    header.extend_from_slice(b"WAVEfmt ");
    header.extend_from_slice(&16u32.to_le_bytes());
    header.extend_from_slice(&1u16.to_le_bytes()); // PCM
    header.extend_from_slice(&1u16.to_le_bytes()); // mono
    header.extend_from_slice(&(RATE as u32).to_le_bytes());
    header.extend_from_slice(&(RATE as u32 * 2).to_le_bytes());
    header.extend_from_slice(&2u16.to_le_bytes());
    header.extend_from_slice(&16u16.to_le_bytes());
    header.extend_from_slice(b"data");
    header.extend_from_slice(&bytes.to_le_bytes());
    file.write_all(&header).expect("write the WAV");
    for s in samples {
        file.write_all(&s.to_le_bytes()).expect("write the WAV");
    }
}

/// A one-line report on `samples`, read `read` samples at a time.
pub fn report(samples: &[i16], read: usize) -> String {
    let x: Vec<f32> = samples.iter().map(|s| f32::from(*s) / 32768.0).collect();
    let level = levels(&x);
    let audible: Vec<f32> = level.iter().copied().filter(|l| *l > -70.0).collect();
    if audible.len() < level.len() / 10 {
        return "silent".into();
    }
    let typical = median(&audible);
    format!(
        "level {typical:5.1} dB | dropouts {} | clicks {} | crackle {:4.1} | read steps {:.2}x (worst {:.1} dB)",
        dropouts(&level).len(),
        clicks(&x),
        median(
            &x.chunks(RATE)
                .filter(|s| s.len() == RATE)
                .map(crackle)
                .collect::<Vec<_>>()
        ),
        read_steps(&level, read).0,
        read_steps(&level, read).1,
    )
}

/// What's wrong with `samples`, against what every bed must do (not clip)
/// and, if `smooth`, what wind and water must (no dropouts, no crackle, no
/// steps at the reads).
pub fn problems(samples: &[i16], read: usize, smooth: bool) -> Vec<String> {
    let mut out = Vec::new();
    let clipped = samples
        .iter()
        .filter(|s| s.unsigned_abs() >= 32_000)
        .count();
    if clipped > 0 {
        out.push(format!("{clipped} samples near full scale"));
    }
    if smooth {
        let x: Vec<f32> = samples.iter().map(|s| f32::from(*s) / 32768.0).collect();
        let level = levels(&x);
        let gaps = dropouts(&level);
        if !gaps.is_empty() {
            out.push(format!("{} dropouts, from {:.2} s", gaps.len(), gaps[0]));
        }
        let crackle = median(
            &x.chunks(RATE)
                .filter(|s| s.len() == RATE)
                .map(crackle)
                .collect::<Vec<_>>(),
        );
        if crackle > 6.0 {
            out.push(format!("crackle {crackle:.1} (smooth noise is 3)"));
        }
        let (steps, _) = read_steps(&level, read);
        if steps > 1.5 {
            out.push(format!("level steps {steps:.2}x as large at the reads"));
        }
    }
    out
}

/// Each 10 ms frame's level, dB.
fn levels(x: &[f32]) -> Vec<f32> {
    x.as_chunks::<FRAME>()
        .0
        .iter()
        .map(|f| 10.0 * (f.iter().map(|v| v * v).sum::<f32>() / FRAME as f32 + 1e-12).log10())
        .collect()
}

fn median(v: &[f32]) -> f32 {
    let mut v = v.to_vec();
    v.sort_by(f32::total_cmp);
    v[v.len() / 2]
}

/// Frames far (18 dB) below the half second round them while that's audible:
/// the sound cutting out. Their times, seconds.
pub fn dropouts(level: &[f32]) -> Vec<f32> {
    (25..level.len().saturating_sub(25))
        .filter(|&k| {
            let around: Vec<f32> = level[k - 25..k - 2]
                .iter()
                .chain(&level[k + 3..k + 25])
                .copied()
                .collect();
            let typical = median(&around);
            typical > -60.0 && level[k] < typical - 18.0
        })
        .map(|k| k as f32 / 100.0)
        .collect()
}

/// The times of `samples`' dropouts, seconds.
pub fn dropout_times(samples: &[i16]) -> Vec<f32> {
    let x: Vec<f32> = samples.iter().map(|s| f32::from(*s) / 32768.0).collect();
    dropouts(&levels(&x))
}

/// Samples whose step from the last is far (12x) above what's usual in the
/// 10 ms round them: discontinuities, which noise and tone don't make.
fn clicks(x: &[f32]) -> usize {
    let d: Vec<f32> = x.windows(2).map(|w| w[1] - w[0]).collect();
    let mut count = 0;
    for frame in d.chunks(FRAME) {
        let rms = (frame.iter().map(|v| v * v).sum::<f32>() / frame.len() as f32).sqrt();
        if rms < 1e-5 {
            continue;
        }
        count += frame.iter().filter(|v| v.abs() > 12.0 * rms).count();
    }
    count
}

/// Kurtosis of the 2 to 8 kHz band, over a second: 3 is smooth noise; sparse
/// ticks, a crackle, run far higher (the leaves' was 34). Rain, fire and lava
/// are made of drops and pops, so theirs is high by nature.
fn crackle(x: &[f32]) -> f32 {
    // RBJ band-pass at 4 kHz, Q 0.6.
    let w = std::f32::consts::TAU * 4000.0 / RATE as f32;
    let alpha = w.sin() / (2.0 * 0.6);
    let a0 = 1.0 + alpha;
    let (b0, b2, a1, a2) = (
        alpha / a0,
        -alpha / a0,
        -2.0 * w.cos() / a0,
        (1.0 - alpha) / a0,
    );
    let (mut x1, mut x2, mut y1, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let (mut m2, mut m4) = (0.0f64, 0.0f64);
    for &v in x {
        let y = b0 * v + b2 * x2 - a1 * y1 - a2 * y2;
        (x2, x1, y2, y1) = (x1, v, y1, y);
        m2 += f64::from(y * y);
        m4 += f64::from(y * y) * f64::from(y * y);
    }
    let n = x.len() as f64;
    if m2 <= 0.0 {
        0.0
    } else {
        ((m4 / n) / (m2 / n).powi(2)) as f32
    }
}

/// How much more the level jumps from one 10 ms frame to the next across a
/// read's start than elsewhere (1x: no sign of the reads), and the worst
/// such jump, dB.
fn read_steps(level: &[f32], read: usize) -> (f32, f32) {
    let step = |k: usize| (level[k] - level[k - 1]).abs();
    let at: Vec<usize> = (1..level.len())
        .filter(|&k| (k * FRAME) % read < FRAME && level[k - 1] > -70.0)
        .collect();
    let elsewhere: Vec<f32> = (1..level.len())
        .filter(|&k| (k * FRAME) % read >= 2 * FRAME && level[k - 1] > -70.0)
        .map(step)
        .collect();
    if at.is_empty() || elsewhere.is_empty() {
        return (1.0, 0.0);
    }
    let mean_at = at.iter().map(|&k| step(k)).sum::<f32>() / at.len() as f32;
    let mean_else = elsewhere.iter().sum::<f32>() / elsewhere.len() as f32;
    let worst = at.iter().map(|&k| step(k)).fold(0.0, f32::max);
    (mean_at / mean_else.max(1e-6), worst)
}
