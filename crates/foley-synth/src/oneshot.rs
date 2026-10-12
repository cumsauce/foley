use crate::rain::Struck;
use crate::{Bubble, LowPass, Rng, SAMPLE_RATE, Surface};

/// A water drip landing in water: the drop's tick, then the bubble it traps
/// ringing at its Minnaert frequency. `radius` is the bubble's, in m.
pub fn drip_into_water(radius: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let mut bubble = Bubble::new(radius, 0.5);
    // The drop's tick as it lands: 2 ms of decaying noise, then the bubble a
    // few milliseconds later, once the cavity pinches off.
    let tick = SAMPLE_RATE as usize / 500;
    let gap = SAMPLE_RATE as usize / 250;
    let mut out = Vec::new();
    for i in 0..tick {
        out.push(0.3 * rng.signed() * (1.0 - i as f32 / tick as f32));
    }
    out.extend(std::iter::repeat_n(0.0, gap));
    while !bubble.done() {
        out.push(bubble.sample());
    }
    out
}

/// A water drip landing on stone: a sharp tick of the stone's modes under a
/// splash, louder the bigger the drop (`radius`, m: its momentum).
pub fn drip_on_stone(radius: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let amplitude = 0.25 * (radius / 0.002).powf(1.5);
    let mut struck = Struck::new(&mut rng, Surface::Stone, amplitude);
    let mut splash = 0.2 * amplitude;
    let splash_step = (-1.0 / (0.004 * SAMPLE_RATE as f32)).exp();
    let mut high = LowPass::new(2500.0);
    let mut out = Vec::new();
    while !struck.done() || splash > 1e-4 {
        let noise = rng.signed();
        let bright = noise - high.process(noise);
        out.push(struck.sample(noise) + splash * bright);
        splash *= splash_step;
    }
    out
}

/// A drop of lava landing: a dull thud, and a sizzle as it meets what's
/// below.
pub fn lava_drip(seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let rate = SAMPLE_RATE as f32;
    let thud_hz = 120.0 + 80.0 * rng.unit();
    let thud_decay = (-1.0 / (0.015 * rate)).exp();
    let sizzle_decay = (-1.0 / (0.12 * rate)).exp();
    let mut thud = 0.6;
    let mut sizzle = 0.12;
    let mut high = LowPass::new(3000.0);
    let length = (0.8 * rate) as usize;
    (0..length)
        .map(|i| {
            let noise = rng.signed();
            let bright = noise - high.process(noise);
            let s =
                thud * (std::f32::consts::TAU * thud_hz * i as f32 / rate).sin() + sizzle * bright;
            thud *= thud_decay;
            sizzle *= sizzle_decay;
            s
        })
        .collect()
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
