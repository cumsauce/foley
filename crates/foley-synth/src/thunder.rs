use crate::{LowPass, Rng, SAMPLE_RATE, absorption_cutoff};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};

const SPEED_OF_SOUND: f32 = 343.0;
const ATMOSPHERE: f32 = 101_325.0;
/// Energy a metre of the return stroke's channel, J/m (Few: 10⁵ to 10⁶).
const ENERGY_PER_METRE: f32 = 5.0e5;
/// Mean change of direction between segments, as Hill (1968) measured it
/// on photographs of lightning: about 16°.
const TORTUOSITY: f32 = 0.38;
/// At most this much thunder plays at once, seconds of audio queued.
const MAX_STRIKES: usize = 6;

/// What the air and the listener's shelter are like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThunderScene {
    pub celsius: f32,
    /// Relative humidity, percent.
    pub humidity: f32,
    /// How open to the sky the listener is: 1 out in the open, 0 shut in,
    /// where only the lows come through the walls.
    pub exposure: f32,
}

impl ThunderScene {
    pub const DEFAULT: ThunderScene = ThunderScene {
        celsius: 15.0,
        humidity: 90.0,
        exposure: 1.0,
    };
}

/// A straight piece of the channel.
#[derive(Debug, Clone, Copy)]
pub struct Segment {
    pub start: [f32; 3],
    /// A unit vector along it.
    pub direction: [f32; 3],
    pub length: f32,
    /// Its energy against the main channel's.
    pub energy: f32,
}

fn add(a: [f32; 3], b: [f32; 3], scale: f32) -> [f32; 3] {
    [
        a[0] + b[0] * scale,
        a[1] + b[1] * scale,
        a[2] + b[2] * scale,
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / n, v[1] / n, v[2] / n]
}

fn random_unit(rng: &mut Rng) -> [f32; 3] {
    loop {
        let v = [rng.signed(), rng.signed(), rng.signed()];
        let n = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        if n > 0.01 && n <= 1.0 {
            return normalize(v);
        }
    }
}

/// A tortuous walk of segments 8 to 38 m long from `start`, `bias` pulling
/// it a way, until it has gone `reach` along `bias` or `length` in all.
fn walk(
    rng: &mut Rng,
    start: [f32; 3],
    first: [f32; 3],
    bias: [f32; 3],
    reach: f32,
    length: f32,
    energy: f32,
) -> Vec<Segment> {
    let mut segments = Vec::new();
    let (mut at, mut direction, mut gone) = (start, first, 0.0);
    let along = |p: [f32; 3]| {
        (p[0] - start[0]) * bias[0] + (p[1] - start[1]) * bias[1] + (p[2] - start[2]) * bias[2]
    };
    while along(at) < reach && gone < length && segments.len() < 2000 {
        direction = normalize(add(add(direction, random_unit(rng), TORTUOSITY), bias, 0.1));
        let step = 8.0 + 30.0 * rng.unit();
        segments.push(Segment {
            start: at,
            direction,
            length: step,
            energy,
        });
        at = add(at, direction, step);
        gone += step;
    }
    segments
}

/// A lightning channel from the ground at `base` up into the cloud, 2.5 to
/// 4 km, with a few branches off its lower part.
pub fn channel(rng: &mut Rng, base: [f32; 3]) -> Vec<Segment> {
    let up = [0.0, 1.0, 0.0];
    let height = 2500.0 + 1500.0 * rng.unit();
    let mut segments = walk(rng, base, up, up, height, 3.0 * height, 1.0);
    let main = segments.len();
    for _ in 0..3 + (rng.unit() * 4.0) as usize {
        let from = segments[(rng.unit() * 0.7 * main as f32) as usize];
        let out = normalize([rng.signed(), -0.3 + 0.4 * rng.signed(), rng.signed()]);
        let length = 150.0 + 550.0 * rng.unit();
        segments.extend(walk(rng, from.start, out, out, length, 2.0 * length, 0.3));
    }
    segments
}

/// The thunder of one flash at `base` (metres from the listener, y up),
/// arriving as it would: the first sound after the nearest part of the
/// channel's distance over the speed of sound.
///
/// The Ribner–Roy model: each segment's shock relaxes into an N-wave,
/// whose length starts at Few's 1/f_m = R₀ / 0.63c (R₀ = √(E / πp), about
/// 1.3 m for the main channel) and grows slowly with distance as a weak shock
/// does. Seen side on, a segment's N-wave arrives all at once, a clap; seen
/// end on, it is smeared over the time sound takes along it (L·|cos φ| / c),
/// a rumble. So the shape of the channel against the listener makes the
/// thunder. Air absorption over each segment's distance (ISO 9613-1) takes
/// the highs off far parts. A flash is several strokes, 30 to 100 ms apart,
/// down the same main channel, weaker each time.
pub fn strike(base: [f32; 3], seed: u64, celsius: f32, humidity: f32) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let segments = channel(&mut rng, base);
    let rate = SAMPLE_RATE as f32;
    let mut strokes = vec![(0.0f32, 1.0f32)];
    let mut offset = 0.0;
    for _ in 0..(rng.unit() * 4.0) as usize {
        offset += 0.03 + 0.07 * rng.unit();
        strokes.push((offset, 0.3 + 0.4 * rng.unit()));
    }
    let mut pulses: Vec<(usize, Vec<f32>)> = Vec::new();
    let mut end = 0usize;
    for segment in &segments {
        let middle = add(segment.start, segment.direction, segment.length / 2.0);
        let r = (middle[0] * middle[0] + middle[1] * middle[1] + middle[2] * middle[2])
            .sqrt()
            .max(1.0);
        let towards = [middle[0] / r, middle[1] / r, middle[2] / r];
        let cos = (segment.direction[0] * towards[0]
            + segment.direction[1] * towards[1]
            + segment.direction[2] * towards[2])
            .abs();
        let smear = segment.length * cos / SPEED_OF_SOUND;
        let cutoff = absorption_cutoff(r, celsius, humidity);
        for &(delay, stroke) in &strokes {
            // Branches carry only the first stroke.
            if delay > 0.0 && segment.energy < 1.0 {
                continue;
            }
            let energy = segment.energy * stroke;
            let r0 = (ENERGY_PER_METRE * energy / (std::f32::consts::PI * ATMOSPHERE)).sqrt();
            let period = r0 / (0.63 * SPEED_OF_SOUND) * (r / (10.0 * r0)).max(1.0).powf(0.25);
            let amplitude = 10.0 * (r0 / 1.25) * segment.length / r;
            let pulse = n_wave(period, smear, amplitude, cutoff);
            let start =
                ((delay + (r - segment.length * cos / 2.0) / SPEED_OF_SOUND) * rate) as usize;
            end = end.max(start + pulse.len());
            pulses.push((start, pulse));
        }
    }
    let mut out = vec![0.0; end + rate as usize / 2];
    for (start, pulse) in pulses {
        for (out, sample) in out[start..].iter_mut().zip(&pulse) {
            *out += sample;
        }
    }
    out
}

/// An N-wave of `period` seconds and peak `amplitude`, smeared over `smear`
/// seconds (averaged over a sliding window that long), then low-passed by
/// the air at `cutoff`.
fn n_wave(period: f32, smear: f32, amplitude: f32, cutoff: Option<f32>) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    // The N-wave's running integral: t - t²/T on [0, T], then 0 (it has no
    // net area). Averaging over a window is a difference of it.
    let integral = |t: f32| {
        if (0.0..period).contains(&t) {
            t - t * t / period
        } else {
            0.0
        }
    };
    let wave = |t: f32| {
        if (0.0..period).contains(&t) {
            1.0 - 2.0 * t / period
        } else {
            0.0
        }
    };
    let tail = cutoff.map_or(0.0, |f| 4.0 / (std::f32::consts::TAU * f));
    let length = ((period + smear + tail) * rate) as usize + 2;
    let window = smear.max(1.0 / rate);
    let mut pulse: Vec<f32> = (0..length)
        .map(|i| {
            let t = i as f32 / rate;
            amplitude
                * if smear < 1.0 / rate {
                    wave(t)
                } else {
                    (integral(t) - integral(t - window)) / window
                }
        })
        .collect();
    if let Some(cutoff) = cutoff {
        let mut filters = [LowPass::new(cutoff), LowPass::new(cutoff)];
        for sample in &mut pulse {
            let first = filters[0].process(*sample);
            *sample = filters[1].process(first);
        }
    }
    pulse
}

/// Thunder (M3): every flash's thunder as it arrives, mixed, all round the
/// listener. Shut in, only the low rumble comes through the walls.
///
/// A flash's thunder takes a few milliseconds and megabytes to make, so it is
/// made on a thread of its own and handed over when ready: rendering, on the
/// audio stream's thread, only ever mixes.
#[derive(Debug)]
pub struct Thunder {
    scene: ThunderScene,
    made: SyncSender<Vec<f32>>,
    ready: Receiver<Vec<f32>>,
    /// Finished thunder, handed back to be freed off the audio thread.
    spent: SyncSender<Vec<f32>>,
    to_free: Receiver<Vec<f32>>,
    /// Flashes whose thunder is still being made.
    making: usize,
    playing: Vec<(Vec<f32>, usize)>,
    muffle: [LowPass; 2],
}

impl Default for Thunder {
    fn default() -> Self {
        Thunder::new()
    }
}

impl Thunder {
    pub fn new() -> Thunder {
        // Bounded, so sending and receiving never allocate: at most
        // `MAX_STRIKES` are ever in flight.
        let (made, ready) = sync_channel(MAX_STRIKES);
        let (spent, to_free) = sync_channel(MAX_STRIKES);
        Thunder {
            scene: ThunderScene::DEFAULT,
            made,
            ready,
            spent,
            to_free,
            making: 0,
            playing: Vec::with_capacity(MAX_STRIKES),
            muffle: [LowPass::new(20_000.0), LowPass::new(20_000.0)],
        }
    }

    pub fn set_scene(&mut self, scene: ThunderScene) {
        self.scene = scene;
        let shut = 1.0 - scene.exposure.clamp(0.0, 1.0);
        let cutoff = 20_000.0 * (200.0f32 / 20_000.0).powf(shut);
        for filter in &mut self.muffle {
            filter.tune(cutoff);
        }
    }

    /// A flash at `base`, metres from the listener (y up), now. Its thunder
    /// is made on a thread of its own and joins the mix when it's ready.
    pub fn strike(&mut self, base: [f32; 3], seed: u64) {
        // Frees finished thunder here, off the audio thread.
        while self.to_free.try_recv().is_ok() {}
        if self.making + self.playing.len() >= MAX_STRIKES {
            return;
        }
        let (made, celsius, humidity) =
            (self.made.clone(), self.scene.celsius, self.scene.humidity);
        let spawned = std::thread::Builder::new()
            .name("foley-thunder".into())
            .spawn(move || {
                // Gone if the bed was let go meanwhile.
                let _ = made.send(strike(base, seed, celsius, humidity));
            });
        if spawned.is_ok() {
            self.making += 1;
        }
    }

    /// Whether nothing is left to hear.
    pub fn quiet(&self) -> bool {
        self.making == 0 && self.playing.is_empty()
    }

    /// Waits for every flash's thunder to be made. For tests.
    #[cfg(test)]
    fn finish_making(&mut self) {
        while self.making > 0 {
            let sound = self.ready.recv().expect("a thunder thread");
            self.making -= 1;
            self.playing.push((sound, 0));
        }
    }

    pub fn render(&mut self, out: &mut [f32]) {
        while let Ok(sound) = self.ready.try_recv() {
            self.making -= 1;
            self.playing.push((sound, 0));
        }
        out.fill(0.0);
        for (sound, at) in &mut self.playing {
            let take = (sound.len() - *at).min(out.len());
            for (out, sample) in out.iter_mut().zip(&sound[*at..*at + take]) {
                *out += sample;
            }
            *at += take;
        }
        let mut k = 0;
        while k < self.playing.len() {
            let (sound, at) = &self.playing[k];
            if *at < sound.len() {
                k += 1;
            } else {
                let (sound, _) = self.playing.swap_remove(k);
                // Never full: it holds at most `MAX_STRIKES`.
                let _ = self.spent.try_send(sound);
            }
        }
        let exposure = self.scene.exposure.clamp(0.0, 1.0);
        if exposure < 0.98 {
            let gain = 0.4 + 0.6 * exposure;
            for sample in out {
                let first = self.muffle[0].process(*sample);
                *sample = gain * self.muffle[1].process(first);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn peak(s: &[f32]) -> f32 {
        s.iter().fold(0f32, |p, x| p.max(x.abs()))
    }

    #[test]
    fn the_channel_is_tortuous_and_reaches_the_cloud() {
        let mut rng = Rng::new(4);
        let segments = channel(&mut rng, [0.0; 3]);
        let main: Vec<_> = segments.iter().filter(|s| s.energy == 1.0).collect();
        let top = main.iter().map(|s| s.start[1]).fold(0f32, f32::max);
        assert!(top > 2400.0, "{top} m");
        let turns: Vec<f32> = main
            .windows(2)
            .map(|w| {
                let d = w[0].direction[0] * w[1].direction[0]
                    + w[0].direction[1] * w[1].direction[1]
                    + w[0].direction[2] * w[1].direction[2];
                d.clamp(-1.0, 1.0).acos().to_degrees()
            })
            .collect();
        let mean = turns.iter().sum::<f32>() / turns.len() as f32;
        assert!((12.0..20.0).contains(&mean), "mean turn {mean}°");
        assert!(segments.len() > main.len(), "branches");
    }

    #[test]
    fn near_thunder_cracks_then_rumbles_and_far_thunder_is_late_and_dull() {
        let rate = SAMPLE_RATE as f32;
        let near = strike([300.0, 0.0, 0.0], 1, 15.0, 90.0);
        // Nothing before sound could arrive: 300 m is 0.87 s.
        let first = near.iter().position(|s| s.abs() > 1e-4).expect("heard") as f32 / rate;
        assert!((0.8..0.9).contains(&first), "first heard at {first} s");
        // A crack near the start, then seconds of rumble from up the channel.
        let crack = peak(&near[..(1.5 * rate) as usize]);
        let far = strike([4000.0, 0.0, 0.0], 1, 15.0, 90.0);
        assert!((0.5..4.0).contains(&crack), "crack {crack}");
        let rumble = peak(&near[(3.0 * rate) as usize..(6.0 * rate) as usize]);
        assert!(rumble > 0.01 && rumble < crack, "rumble {rumble}");
        assert!(
            near.len() as f32 / rate > 6.0,
            "{} s",
            near.len() as f32 / rate
        );
        assert!(peak(&far) < 0.3 * crack, "far {}", peak(&far));
        // Duller: fewer zero crossings a second in its loudest stretch.
        let crossings = |s: &[f32]| {
            s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f32 / s.len() as f32
        };
        let loudest = |s: &[f32]| {
            let i = s
                .iter()
                .enumerate()
                .fold(
                    (0, 0f32),
                    |b, (i, x)| if x.abs() > b.1 { (i, x.abs()) } else { b },
                )
                .0;
            s[i.saturating_sub(4800)..(i + 4800).min(s.len())].to_vec()
        };
        assert!(crossings(&loudest(&far)) < crossings(&loudest(&near)));
    }

    #[test]
    fn indoors_only_the_lows_come_through() {
        let mut open = Thunder::new();
        let mut shut = Thunder::new();
        shut.set_scene(ThunderScene {
            exposure: 0.0,
            ..ThunderScene::DEFAULT
        });
        for thunder in [&mut open, &mut shut] {
            thunder.strike([300.0, 0.0, 0.0], 2);
            thunder.finish_making();
        }
        let mut a = vec![0.0; 3 * SAMPLE_RATE as usize];
        let mut b = a.clone();
        open.render(&mut a);
        shut.render(&mut b);
        assert!(peak(&b) < 0.5 * peak(&a));
        assert!(!open.quiet());
        let mut rest = vec![0.0; 20 * SAMPLE_RATE as usize];
        open.render(&mut rest);
        assert!(open.quiet());
    }
}
