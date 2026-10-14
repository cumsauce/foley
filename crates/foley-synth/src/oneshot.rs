use crate::dsp::add_at;
use crate::{Bubble, LowPass, Rng, SAMPLE_RATE, Surface};
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
}
