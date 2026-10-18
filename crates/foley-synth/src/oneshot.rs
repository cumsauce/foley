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
pub fn drip_into_water(radius: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let rate = SAMPLE_RATE as f32;
    let mut out = click(0.002, 6000.0, 1.2 * (0.6 + 0.4 * rng.unit()), &mut rng);
    if rng.unit() < 0.15 {
        // No bubble: a little splash.
        let splash = click(0.03, 3000.0, 0.5, &mut rng);
        add_at(&mut out, (0.002 * rate) as usize, &splash, 1.0);
    } else {
        // The bubble, a few milliseconds later, once the cavity pinches off.
        let mut bubble = Bubble::new(radius, 0.5);
        let sound = ring(&mut bubble);
        add_at(&mut out, (0.006 * rate) as usize, &sound, 1.0);
    }
    if rng.unit() < 0.45 {
        let at = ((0.06 + 0.08 * rng.unit()) * rate) as usize;
        let tick = click(0.001, 8000.0, 0.4, &mut rng);
        add_at(&mut out, at, &tick, 1.0);
        let mut droplet = Bubble::new(radius * (0.4 + 0.2 * rng.unit()), 0.2);
        let sound = ring(&mut droplet);
        add_at(&mut out, at + (0.003 * rate) as usize, &sound, 1.0);
    }
    out
}

/// A water drip landing on stone: a sharp tick of the stone's modes under a
/// splash, louder the bigger the drop (`radius`, m: its momentum). The drop
/// bursts into a crown whose droplets spatter round it over the next few
/// tens of milliseconds; on a wet patch, now and then one traps a bubble.
pub fn drip_on_stone(radius: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let rate = SAMPLE_RATE as f32;
    let amplitude = 0.25 * (radius / 0.002).powf(1.5);
    let mut struck = Struck::new(&mut rng, Surface::Stone, amplitude);
    let mut splash = 0.2 * amplitude;
    let splash_step = (-1.0 / (0.004 * rate)).exp();
    let mut high = LowPass::new(2500.0);
    let mut out = Vec::new();
    while !struck.done() || splash > 1e-4 {
        let noise = rng.signed();
        let bright = noise - high.process(noise);
        out.push(struck.sample(noise) + splash * bright);
        splash *= splash_step;
    }
    for _ in 0..3 + (rng.unit() * 6.0) as usize {
        let at = ((0.005 + 0.055 * rng.unit()) * rate) as usize;
        let tick = click(
            0.0008,
            9000.0,
            amplitude * (0.05 + 0.15 * rng.unit()),
            &mut rng,
        );
        add_at(&mut out, at, &tick, 1.0);
    }
    if rng.unit() < 0.25 {
        let mut bubble = Bubble::new(0.0006 + 0.0006 * rng.unit(), 0.1 * amplitude);
        let sound = ring(&mut bubble);
        add_at(
            &mut out,
            ((0.01 + 0.03 * rng.unit()) * rate) as usize,
            &sound,
            1.0,
        );
    }
    out
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
