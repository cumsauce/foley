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
/// and, if `smooth`, what wind and water must (no clicks, no dropouts, no
/// crackle, no steps at the reads).
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
        let ticks = clicks(&x);
        if ticks > 0 {
            out.push(format!("{ticks} clicks"));
        }
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

/// One bed's numbers, for the report, the baseline and the plot.
#[derive(Debug, Clone, Copy, Default)]
pub struct Metrics {
    /// Integrated loudness, LUFS (ITU-R BS.1770: K-weighted, gated at -70).
    pub lufs: f32,
    pub dropouts: usize,
    pub clicks: usize,
    pub crackle: f32,
    pub read_steps: f32,
}

impl Metrics {
    pub fn of(samples: &[i16], read: usize) -> Metrics {
        let x: Vec<f32> = samples.iter().map(|s| f32::from(*s) / 32768.0).collect();
        let level = levels(&x);
        let crackles: Vec<f32> = x
            .chunks(RATE)
            .filter(|s| s.len() == RATE)
            .map(crackle)
            .collect();
        Metrics {
            lufs: lufs(&x),
            dropouts: dropouts(&level).len(),
            clicks: clicks(&x),
            crackle: if crackles.is_empty() {
                0.0
            } else {
                median(&crackles)
            },
            read_steps: read_steps(&level, read).0,
        }
    }

    /// A line of the baseline file: `name lufs dropouts clicks crackle steps`.
    pub fn line(&self, name: &str) -> String {
        format!(
            "{name}\t{:.2}\t{}\t{}\t{:.2}\t{:.2}",
            self.lufs, self.dropouts, self.clicks, self.crackle, self.read_steps
        )
    }

    pub fn parse(line: &str) -> Option<(String, Metrics)> {
        let mut f = line.split('\t');
        let name = f.next()?.to_string();
        let mut num = || f.next().and_then(|v| v.parse::<f32>().ok());
        Some((
            name,
            Metrics {
                lufs: num()?,
                dropouts: num()? as usize,
                clicks: num()? as usize,
                crackle: num()?,
                read_steps: num()?,
            },
        ))
    }

    /// What changed from `before` past what chance moves: loudness by 1 LU,
    /// crackle by 1.5, read steps by 0.3, any dropout or click.
    pub fn changes(&self, before: &Metrics) -> Vec<String> {
        let mut out = Vec::new();
        if (self.lufs - before.lufs).abs() >= 1.0 {
            out.push(format!(
                "loudness {:+.1} LU ({:.1} to {:.1} LUFS)",
                self.lufs - before.lufs,
                before.lufs,
                self.lufs
            ));
        }
        if (self.crackle - before.crackle).abs() >= 1.5 {
            out.push(format!(
                "crackle {:.1} to {:.1}",
                before.crackle, self.crackle
            ));
        }
        if (self.read_steps - before.read_steps).abs() >= 0.3 {
            out.push(format!(
                "read steps {:.2}x to {:.2}x",
                before.read_steps, self.read_steps
            ));
        }
        if self.dropouts != before.dropouts {
            out.push(format!("dropouts {} to {}", before.dropouts, self.dropouts));
        }
        if self.clicks != before.clicks {
            out.push(format!("clicks {} to {}", before.clicks, self.clicks));
        }
        out
    }
}

/// Integrated loudness (ITU-R BS.1770-4 at 48 kHz, mono): K-weighted (a
/// shelf for the head, a high-pass), in 400 ms blocks overlapping by 75 %,
/// gated at -70 LUFS and then 10 LU under the ungated mean.
fn lufs(x: &[f32]) -> f32 {
    let stages = [
        (
            [1.535_124_9, -2.691_696_2, 1.198_392_8],
            [-1.690_659_3, 0.732_480_8],
        ),
        ([1.0, -2.0, 1.0], [-1.990_047_5, 0.990_072_25]),
    ];
    let mut y: Vec<f32> = x.to_vec();
    for (b, a) in stages {
        let (mut x1, mut x2, mut y1, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for v in &mut y {
            let out = b[0] * *v + b[1] * x1 + b[2] * x2 - a[0] * y1 - a[1] * y2;
            (x2, x1, y2, y1) = (x1, *v, y1, out);
            *v = out;
        }
    }
    let block = RATE * 4 / 10;
    let hop = block / 4;
    let powers: Vec<f64> = (0..y.len().saturating_sub(block) / hop)
        .map(|k| {
            y[k * hop..k * hop + block]
                .iter()
                .map(|v| f64::from(*v).powi(2))
                .sum::<f64>()
                / block as f64
        })
        .collect();
    let loud = |p: f64| -0.691 + 10.0 * p.max(1e-12).log10();
    let gated: Vec<f64> = powers
        .iter()
        .copied()
        .filter(|p| loud(*p) > -70.0)
        .collect();
    if gated.is_empty() {
        return -70.0;
    }
    let mean = gated.iter().sum::<f64>() / gated.len() as f64;
    let relative: Vec<f64> = gated
        .iter()
        .copied()
        .filter(|p| loud(*p) > loud(mean) - 10.0)
        .collect();
    loud(relative.iter().sum::<f64>() / relative.len().max(1) as f64) as f32
}

/// An SVG of a bed's level (100 ms steps) under its scene's params (each
/// scaled to its own range), with the reads and any dropouts marked.
pub fn plot(samples: &[i16], read: usize, params: &[(f32, Vec<f32>)], title: &str) -> String {
    let x: Vec<f32> = samples.iter().map(|s| f32::from(*s) / 32768.0).collect();
    let level = levels(&x);
    let seconds = samples.len() as f32 / RATE as f32;
    let (w, h, top) = (1200.0f32, 360.0f32, 40.0f32);
    let px = |t: f32| 40.0 + (w - 60.0) * t / seconds;
    let py = |db: f32| top + (h - top - 30.0) * (-db.clamp(-80.0, 0.0)) / 80.0;
    let mut svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}' font-family='sans-serif' font-size='11'>\
         <rect width='100%' height='100%' fill='white'/><text x='40' y='20' font-size='14'>{title}</text>"
    );
    for db in [0, -20, -40, -60, -80] {
        let y = py(db as f32);
        svg += &format!(
            "<line x1='40' x2='{}' y1='{y}' y2='{y}' stroke='#eee'/><text x='4' y='{}'>{db} dB</text>",
            w - 20.0,
            y + 4.0
        );
    }
    let reads = samples.len() / read;
    if reads <= 400 {
        for k in 1..=reads {
            let t = (k * read) as f32 / RATE as f32;
            svg += &format!(
                "<line x1='{0}' x2='{0}' y1='{top}' y2='{1}' stroke='#f2f2ff'/>",
                px(t),
                h - 30.0
            );
        }
    }
    let colors = ["#2a9d8f", "#e9c46a", "#f4a261", "#9b5de5", "#00bbf9"];
    let count = params.first().map_or(0, |p| p.1.len()).min(colors.len());
    for (i, color) in colors.iter().enumerate().take(count) {
        let most = params.iter().map(|p| p.1[i].abs()).fold(1e-6, f32::max);
        let points: Vec<String> = params
            .iter()
            .map(|(t, p)| {
                format!(
                    "{:.1},{:.1}",
                    px(*t),
                    top + (h - top - 30.0) * (1.0 - p[i] / most)
                )
            })
            .collect();
        svg += &format!(
            "<polyline fill='none' stroke='{color}' stroke-dasharray='4 3' points='{}'/>",
            points.join(" ")
        );
        svg += &format!(
            "<text x='{}' y='{}' fill='{color}'>param {i} (to {most:.1})</text>",
            w - 160.0,
            top + 14.0 * i as f32
        );
    }
    let points: Vec<String> = level
        .chunks(10)
        .enumerate()
        .map(|(k, f)| {
            format!(
                "{:.1},{:.1}",
                px(k as f32 / 10.0),
                py(f.iter().copied().fold(-120.0, f32::max))
            )
        })
        .collect();
    svg += &format!(
        "<polyline fill='none' stroke='#264653' points='{}'/>",
        points.join(" ")
    );
    for t in dropouts(&level) {
        svg += &format!(
            "<circle cx='{}' cy='{}' r='5' fill='none' stroke='red'/>",
            px(t),
            py(-75.0)
        );
    }
    svg += &format!(
        "<text x='{}' y='{}'>{seconds:.0} s</text></svg>",
        w - 50.0,
        h - 10.0
    );
    svg
}
