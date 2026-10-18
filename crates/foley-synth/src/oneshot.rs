use crate::dsp::add_at;
use crate::{Bubble, LowPass, Rng, SAMPLE_RATE, Surface, noise_burst};
use foley_dsp::Struck;

/// A short click: `seconds` of noise through a low-pass at `cutoff`,
/// decaying.
fn click(seconds: f32, cutoff: f32, amplitude: f32, rng: &mut Rng) -> Vec<f32> {
    let n = ((seconds * SAMPLE_RATE as f32) as usize).max(2);
    let mut low = LowPass::new(cutoff);
    (0..n)
        .map(|i| amplitude * (1.0 - i as f32 / n as f32).powi(2) * low.process(rng.signed()))
        .collect()
}

fn ring(bubble: &mut Bubble) -> Vec<f32> {
    let mut out = Vec::new();
    while !bubble.done() {
        out.push(bubble.sample());
    }
    out
}

/// A water drip landing in water: the drop's tick, then the bubble it traps
/// ringing at its Minnaert frequency. `radius` is the bubble's, in m. Not
/// every drop traps one (only those landing in the right range of speed and
/// size do): some only splash. Often the column of water that springs back
/// up (the Worthington jet) throws off a droplet, which lands a moment later
/// with a smaller plink of its own.
/// A water drip landing in water: the drop's tick, then the bubble it traps
/// ringing at its Minnaert frequency. `radius` is the bubble's, in m — the
/// size the drop traps, so the pitch and the ring's length both follow the
/// drop. Not every drop traps one: some only splash. A bigger bubble rings
/// louder, lower and longer. The cavity's own ring is a few milliseconds at
/// the sizes a drip traps (3 mm rings for ~6 ms), so the drop's tail is
/// carried by the water it sets moving: a short reflection off the surface
/// and, often, a second droplet the crown throws off — which is what makes
/// it read as a drop landing rather than as a ping.
pub fn drip_into_water(radius: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let rate = SAMPLE_RATE as f32;
    // The entry: the drop breaking the surface, eased in over 0.15 ms so the
    // first sample is a rise and not a step the speaker reproduces as a click.
    let mut contact = click(0.0015, 5000.0, 0.12, &mut rng);
    let ease = (0.00015 * rate) as usize;
    for (i, x) in contact.iter_mut().enumerate() {
        if i < ease {
            *x *= i as f32 / ease as f32;
        }
    }
    let mut out = contact;
    // The cavity pinches off at 1.2 ms, before the splash has faded, so
    // the bubble and the entry are one sound and not two. Every drip traps
    // one: a drop that only splashed was a whisper of a 12 ms tick, which
    // read as a glitch rather than as water. Variants differ in size and
    // in the droplets that follow, never in whether they sound at all.
    let mut bubble = Bubble::new(radius, 1.0);
    let ringing = ring(&mut bubble);
    add_at(&mut out, (0.0012 * rate) as usize, &ringing, 1.0);
    // The surface answering: a quiet, low-passed reflection of the cavity
    // collapsing, ~6 ms behind the bubble and about a third as loud. This
    // is the mass under the ping.
    let mut echo = Bubble::new(radius * 1.6, 0.3).with_rise(0.0);
    let mut reflected = ring(&mut echo);
    reflected.iter_mut().for_each(|x| *x *= 0.6);
    add_at(&mut out, (0.0072 * rate) as usize, &reflected, 1.0);
    // Now and then the column springing back (the Worthington jet) throws off
    // a droplet, which lands a moment later with a small plink of its own —
    // the second "drip" the ear expects from a real drop.
    if rng.unit() < 0.75 {
        let at = ((0.026 + 0.030 * rng.unit()) * rate) as usize;
        let mut droplet = Bubble::new(radius * (0.45 + 0.25 * rng.unit()), 0.55);
        let plink = ring(&mut droplet);
        add_at(&mut out, at, &plink, 1.0);
    }
    // Level follows the drop, peak-normalised so no variant distorts, with the
    // last 3 ms faded so nothing clicks off.
    fade_into_level(out, drip_level(radius), 0.003)
}

/// A water drip landing on stone: a sharp tick of the stone's modes under a
/// splash, louder the bigger the drop (`radius`, m: its momentum). The drop
/// bursts into a crown whose droplets spatter round it over the next few
/// tens of milliseconds; on a wet patch, now and then one traps a bubble.
pub fn drip_on_stone(radius: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let rate = SAMPLE_RATE as f32;
    // The level follows the drop's momentum (r^1.5) but slowly: 1 mm is not
    // handed a whisper and 3 mm is not handed a shout.
    let amplitude = 0.25 * ((radius / 0.002).powf(1.5)).powf(0.35);
    // The contact itself lasts ~4 ms (a water film cushions it), and the
    // splash over it ~20 ms: the drop has mass, so it is not a single tick.
    let mut struck = Struck::new(&mut rng, Surface::Stone, amplitude);
    let contact = (0.004 * rate) as usize;
    let mut splash = 0.2 * amplitude;
    let splash_step = (-1.0 / (0.020 * rate)).exp();
    let mut high = LowPass::new(1800.0);
    let mut out = Vec::new();
    while !struck.done() || splash > 1e-4 {
        let noise = rng.signed();
        // A softer slope (1.8 kHz) than before: the 4–9 kHz band is the thin,
        // hissy part of the old drip. The contact is windowed over the first
        // few milliseconds so the stone's modes swell instead of clicking on.
        let bright = noise - high.process(noise);
        let i = out.len();
        let swell = if i < contact {
            0.5 - 0.5
                * std::f32::consts::PI
                    .mul_add(i as f32 / contact as f32, 0.0)
                    .cos()
        } else {
            1.0
        };
        out.push((struck.sample(noise) + splash * bright) * swell);
        splash *= splash_step;
    }
    // The crown's droplets spatter over ~80 ms and, now and then, one traps a
    // bubble of its own.
    for _ in 0..3 + (rng.unit() * 6.0) as usize {
        let at = ((0.008 + 0.070 * rng.unit().powi(2)) * rate) as usize;
        let tick = click(
            0.0008,
            7000.0,
            amplitude * (0.05 + 0.10 * rng.unit()),
            &mut rng,
        );
        add_at(&mut out, at, &tick, 1.0);
    }
    if rng.unit() < 0.35 {
        let mut bubble = Bubble::new(0.0006 + 0.0010 * rng.unit(), 0.35 * amplitude);
        let sound = ring(&mut bubble);
        add_at(
            &mut out,
            ((0.010 + 0.040 * rng.unit()) * rate) as usize,
            &sound,
            1.0,
        );
    }
    // The same level law as the water drip (times the seed's slight variation,
    // so the sixteen variants are not all equally loud on stone), the last 4 ms
    // faded so it never clicks off.
    let variation = 0.88 + 0.12 * Rng::new(seed ^ 0xA11CE).unit();
    fade_into_level(out, drip_level(radius) * variation, 0.004)
}

/// Scales `out` to `level`'s peak (or down to 0.9 if `level` would clip) and
/// fades the last `fade_seconds` off to zero, so a one-shot never ends on a
/// step the speaker reproduces as a click. Every drip goes through it.
fn fade_into_level(mut out: Vec<f32>, level: f32, fade_seconds: f32) -> Vec<f32> {
    let length = out.len();
    let fade = ((fade_seconds * SAMPLE_RATE as f32) as usize).min(length);
    for i in 0..fade {
        out[length - 1 - i] *= i as f32 / fade as f32;
    }
    crate::dsp::normalize_in_place(&mut out, level.min(0.9));
    out
}

/// A drip's peak level for a drop of `radius` (m): how loud the event is
/// before normalisation, so a 1 mm drop is quieter than a 3 mm one by a
/// fixed, gentle law rather than by whichever branch its seed took. Shared
/// by both drip families so they sit together in a cave.
fn drip_level(radius: f32) -> f32 {
    // 0.55 at 0.6 mm, 0.90 at 1.6 mm, and still 0.90 beyond: the level follows
    // the drop's mass (r^1.5) but compressed, so the family stays mixed. The
    // smallest drips used to sit at 0.16 and were lost under the cave bed.
    (0.55 * (radius / 0.0006).powf(0.55)).min(0.9)
}

/// A drop of lava landing: a dull thud, falling in pitch as the blob
/// spreads, a sizzle as it meets what's below, its skin crackling as it
/// cools, and gas bubbles working their way out and popping.
pub fn lava_drip(seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let rate = SAMPLE_RATE as f32;
    let thud_hz = 120.0 + 80.0 * rng.unit();
    let thud_decay = (-1.0 / (0.015 * rate)).exp();
    let sizzle_decay = (-1.0 / (0.12 * rate)).exp();
    let mut thud = 0.6;
    let mut sizzle = 0.12;
    let mut high = LowPass::new(3000.0);
    let mut phase = 0.0f32;
    let length = (0.8 * rate) as usize;
    let mut out: Vec<f32> = (0..length)
        .map(|i| {
            let t = i as f32 / rate;
            let noise = rng.signed();
            let bright = noise - high.process(noise);
            phase += std::f32::consts::TAU * thud_hz * (1.0 - 0.25 * (t / 0.05).min(1.0)) / rate;
            let s = thud * phase.sin() + sizzle * bright;
            thud *= thud_decay;
            sizzle *= sizzle_decay;
            s
        })
        .collect();
    // The skin crackling: clicks thinning out as it cools.
    for _ in 0..20 + (rng.unit() * 20.0) as usize {
        let at = (0.6 * rng.unit().powi(2) * rate) as usize;
        let tick = click(0.0006, 7000.0, 0.04 + 0.08 * rng.unit(), &mut rng);
        add_at(&mut out, at, &tick, 1.0);
    }
    // Gas bubbles popping through the blob: low, quick "blups".
    for _ in 0..2 + (rng.unit() * 4.0) as usize {
        let at = ((0.08 + 0.6 * rng.unit()) * rate) as usize;
        let hz = 60.0 + 90.0 * rng.unit();
        let n = (0.06 * rate) as usize;
        let mut p = 0.0f32;
        let blup: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / n as f32;
                p += std::f32::consts::TAU * hz * (1.0 + 0.6 * t) / rate;
                0.25 * (t * 20.0).min(1.0) * (1.0 - t).powi(3) * p.sin()
            })
            .collect();
        add_at(&mut out, at, &blup, 1.0);
    }
    out.truncate(length);
    let fade = (0.05 * rate) as usize;
    for i in 0..fade {
        out[length - 1 - i] *= i as f32 / fade as f32;
    }
    out
}

/// An explosion's bang (M27): the sub boom of the blast's pressure wave
/// falling in pitch, two layers of blast noise — the low body and the
/// broadband crack — and debris scattering through the tail. `size_m` is the
/// explosion's radius in metres (0.5 to 20; anything else falls back to 2):
/// a bigger blast booms lower, decays slower, runs longer — `min(3.0,
/// 0.6 + 0.25*size)` seconds — and throws more debris. Deterministic from
/// `seed`, peak-normalized to 0.9.
pub fn bang(size_m: f32, seed: u64) -> Vec<f32> {
    let size = if size_m.is_finite() && size_m > 0.0 {
        size_m
    } else {
        2.0
    }
    .clamp(0.5, 20.0);
    let mut rng = Rng::new(seed);
    let rate = SAMPLE_RATE as f32;
    let seconds = (0.6 + 0.25 * size).min(3.0);
    let length = (seconds * rate) as usize;
    let mut out = vec![0.0f32; length];

    // The sub boom: the pressure wave's low thump, its frequency falling
    // exponentially from 110/√size to 35/√size Hz over the first 15% of the
    // blast and then holding, amplitude decaying with tau = 0.12 + 0.05*size.
    let start_hz = 110.0 / size.sqrt();
    let end_hz = 35.0 / size.sqrt();
    let glide = 0.15 * seconds;
    let tau = 0.12 + 0.05 * size;
    let mut phase = 0.0f32;
    for (i, sample) in out.iter_mut().enumerate() {
        let t = i as f32 / rate;
        let hz = if t < glide {
            start_hz * (end_hz / start_hz).powf(t / glide)
        } else {
            end_hz
        };
        phase += std::f32::consts::TAU * hz / rate;
        *sample += (-t / tau).exp() * phase.sin();
    }

    // The blast body: the low shove of the displaced air, then the
    // broadband crack of the shock front itself, both from the blast's
    // first moments.
    let span = 0.25 + 0.05 * size;
    let root = size.sqrt();
    let layers = [
        (140.0 / root, 0.8, span, 1.0, seed.wrapping_add(1)),
        (900.0 / root, 0.7, span, 0.7, seed.wrapping_add(2)),
    ];
    for (centre, q, seconds, gain, layer_seed) in layers {
        let burst = noise_burst(centre, q, seconds, gain, layer_seed);
        for (i, sample) in burst.iter().enumerate() {
            if let Some(slot) = out.get_mut(i) {
                *slot += sample;
            }
        }
    }

    // Debris in the tail: short, bright ticks of fragments landing, at
    // seeded times between the 20%-point and the end of the buffer.
    for i in 0..(4.0 + 2.0 * size) as usize {
        let tick_seconds = 0.01 + 0.02 * rng.unit();
        let centre = 2000.0 + 4000.0 * rng.unit();
        let q = 2.0 + 4.0 * rng.unit();
        let gain = 0.1 + 0.15 * rng.unit();
        let tick = noise_burst(
            centre,
            q,
            tick_seconds,
            gain,
            seed.wrapping_add(0x9E37_79B9).wrapping_add(i as u64),
        );
        let from = 0.2 * seconds;
        let room = (seconds - from - tick_seconds).max(0.0);
        let at = ((from + room * rng.unit()) * rate) as usize;
        for (j, sample) in tick.iter().enumerate() {
            if let Some(slot) = out.get_mut(at + j) {
                *slot += sample;
            }
        }
    }

    // End soft, so the buffer never clicks off, then peak at 0.9.
    let fade = ((0.01 * rate) as usize).min(length);
    for i in 0..fade {
        out[length - 1 - i] *= i as f32 / fade as f32;
    }
    crate::dsp::normalize_in_place(&mut out, 0.9);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The drip model still holds water: a drip rings and ends, a small drop
    /// is quieter than a big one, and lava's drip dies away.
    #[test]
    fn drips_ring_and_end() {
        let water = drip_into_water(0.001, 7);
        assert!(water.len() > 900 && water.len() < SAMPLE_RATE as usize);
        let stone = drip_on_stone(0.002, 7);
        assert!(
            !stone.is_empty() && stone.len() < SAMPLE_RATE as usize,
            "{}",
            stone.len()
        );
        let peak = |s: &[f32]| s.iter().fold(0f32, |p, x| p.max(x.abs()));
        assert!(peak(&stone) > 0.05);
        // A smaller drop is quieter.
        assert!(peak(&drip_on_stone(0.001, 7)) < peak(&stone));
        let lava = lava_drip(3);
        assert!(peak(&lava) > 0.1 && peak(&lava[lava.len() - 100..]) < 0.01);
    }

    #[test]
    fn drips_are_normalised_continuous_and_sized_by_the_drop() {
        let peak = |s: &[f32]| s.iter().fold(0f32, |p, x| p.max(x.abs()));
        for seed in 0..16u64 {
            for radius in [0.0006f32, 0.0015, 0.003] {
                let water = drip_into_water(radius, 0x5EED + seed);
                assert!(!water.is_empty());
                assert!(water.iter().all(|x| x.is_finite()));
                // The level is the drop's, and only the drop's: the same
                // radius gives the same peak whatever branch the seed took,
                // and no variant ever clips into the mixer.
                assert!(
                    (peak(&water) - drip_level(radius)).abs() < 1e-3,
                    "peak {} for radius {radius}, expected {}",
                    peak(&water),
                    drip_level(radius)
                );
                assert!(peak(&water) <= 1.0, "no clipping into the mixer");
                // No hole at the start: not one silent sample before the event.
                let first = water.iter().position(|x| *x != 0.0).unwrap_or(usize::MAX);
                assert!(first < 2, "the drip starts with {first} silent samples");
            }
            let stone = drip_on_stone(0.002, 0xD121 + seed);
            // The stone family varies its level slightly per seed (so sixteen
            // variants are not all equally loud), between 88% and 100% of the
            // shared law — and never above it.
            let ceiling = drip_level(0.002);
            assert!(
                peak(&stone) >= 0.88 * ceiling - 1e-3 && peak(&stone) <= ceiling + 1e-3,
                "peak {} for ceiling {ceiling}",
                peak(&stone)
            );
            let first = stone.iter().position(|x| *x != 0.0).unwrap_or(usize::MAX);
            assert!(
                first < 2,
                "the stone drip starts with {first} silent samples"
            );
        }
        // A bigger drop is louder than a small one, by the shared law — not
        // quieter because its seed happened to skip the bubble. The law is
        // deliberately compressed (0.55 at 0.6 mm to 0.90 at 1.6 mm), so this
        // pins the direction and the audible margin, not a raw ratio.
        let quiet = peak(&drip_into_water(0.0006, 0x5EED));
        let loud = peak(&drip_into_water(0.003, 0x5EED));
        assert!(loud > quiet * 1.5, "{quiet} vs {loud}");
        let quiet = peak(&drip_on_stone(0.001, 0xD121));
        let loud = peak(&drip_on_stone(0.003, 0xD121));
        assert!(loud > quiet, "{quiet} vs {loud}");
    }

    /// A drip has mass: it runs well past the initial contact, and it sits in
    /// the ear's most sensitive band rather than up in the hiss.
    #[test]
    fn a_drip_has_body_that_outlasts_the_contact() {
        let share = |s: &[f32], ms: f32| {
            let n = ((ms / 1000.0) * SAMPLE_RATE as f32) as usize;
            let e0: f32 = s[..n.min(s.len())].iter().map(|x| x * x).sum();
            let all: f32 = s.iter().map(|x| x * x).sum();
            e0 / all
        };
        // A drip is not over in 20 ms: the bubble rings and the water settles
        // after it. At the drops a dripstone actually releases (2–3 mm) the
        // event runs 40–80 ms, and a good fifth of its energy is past 10 ms.
        for seed in 0..16u64 {
            let water = drip_into_water(0.003, 0x5EED + seed);
            assert!(water.len() >= 40 * 48, "only {} ms", water.len() / 48);
            assert!(
                share(&water, 10.0) < 0.9,
                "only {:.1}% of the drip lasts past 10 ms",
                (1.0 - share(&water, 10.0)) * 100.0
            );
            let stone = drip_on_stone(0.0024, 0xD121 + seed);
            assert!(stone.len() >= 40 * 48, "only {} ms", stone.len() / 48);
            assert!(
                share(&stone, 10.0) < 0.8,
                "only {:.1}% of the stone drip lasts past 10 ms",
                (1.0 - share(&stone, 10.0)) * 100.0
            );
        }
        // A stone drip is audible on tiny speakers: not all of its energy up
        // above 4 kHz, where a phone plays nothing.
        let stone = drip_on_stone(0.0024, 3);
        let n = 1024;
        let seg = &stone[..n.min(stone.len())];
        let goertzel = |hz: f32| {
            let w = std::f32::consts::TAU * hz / SAMPLE_RATE as f32;
            seg.iter()
                .enumerate()
                .map(|(i, x)| x * (w * i as f32).cos())
                .sum::<f32>()
                .abs()
        };
        let low = goertzel(1200.0) + goertzel(2000.0);
        let high = goertzel(6000.0) + goertzel(8000.0);
        assert!(low > 0.5 * high, "low {low} vs high {high}: all hiss");
    }

    /// The bang contract: bounded, finite, peak-normalized, seeded — and the
    /// duration law, a bigger blast running longer (until the 3 s cap).
    #[test]
    fn bang_is_seeded_bounded_and_grows_with_the_blast() {
        let small = bang(1.0, 7);
        assert!(!small.is_empty());
        assert!(small.len() <= 3 * SAMPLE_RATE as usize, "{}", small.len());
        assert!(small.iter().all(|x| x.is_finite()));
        let peak = small.iter().fold(0f32, |p, x| p.max(x.abs()));
        assert!((peak - 0.9).abs() < 1e-3, "peak {peak}");
        // The same seed makes the same bang, exactly.
        assert_eq!(small, bang(1.0, 7));
        // A bigger explosion runs longer...
        let big = bang(6.0, 7);
        assert!(big.len() > small.len(), "{} vs {}", big.len(), small.len());
        assert!(big.len() <= 3 * SAMPLE_RATE as usize);
        // ...and an unusable size is heard as a 2 m one, while the buffer
        // never runs past 3 s however large the caller's number.
        assert_eq!(bang(f32::NAN, 7), bang(2.0, 7));
        assert_eq!(bang(0.0, 7), bang(2.0, 7));
        assert_eq!(bang(-3.0, 7), bang(2.0, 7));
        assert_eq!(bang(1e9, 7).len(), (3.0 * SAMPLE_RATE as f32) as usize);
    }
}
