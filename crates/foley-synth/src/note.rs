//! One note block hit: the instrument table mapping a vanilla instrument
//! index to the family that models it and the base frequency (Hz) of its
//! note 12, the pitch law, and — as this plan lands — one synthesiser per
//! family. The index order is Foley's own; the Java twin is
//! `NoteBlocks.INSTRUMENTS`.

use crate::LowPass;
use crate::Rng;
use crate::SAMPLE_RATE;
use std::f32::consts::LN_10;
use std::f32::consts::TAU;

/// Which family models an instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Pluck,
    Modal,
    Membrane,
    Breath,
    Electronic,
}

/// A struck instrument's modal voicing: how far its modes sit off the
/// harmonic series (0 near-harmonic bar, 1 clanking metal) and how long the
/// strike rings (0 dies almost at once, 1 rings on for seconds).
#[derive(Debug, Clone, Copy)]
struct Voicing {
    inharmonicity: f32,
    damping: f32,
}

/// The voicing of an instrument that is not modelled modally; only
/// `Family::Modal` looks at it.
const NO_VOICING: Voicing = Voicing {
    inharmonicity: 0.0,
    damping: 0.0,
};

/// The vanilla note block instruments, in Foley's index order (0..=15),
/// each with the family that models it and the base frequency (Hz) of its
/// note 12. The order is Foley's own; the Java twin is `NoteBlocks.INSTRUMENTS`.
///
/// The order is the `NoteBlockInstrument` constant order in the mapped
/// Minecraft 26.3 sources (verified against the enum's `<clinit>`: HARP,
/// BASEDRUM, SNARE, HAT, BASS, FLUTE, BELL, GUITAR, CHIME, XYLOPHONE,
/// IRON_XYLOPHONE, COW_BELL, DIDGERIDOO, BIT, BANJO, PLING). 26.3 adds
/// trumpets and mob-head imitations after PLING; Foley models the classic
/// sixteen, so those fall outside the table. The base frequencies are
/// Foley's own voicings, pinned per family with a measured fundamental in
/// later tasks of this plan. The struck instruments also carry their modal
/// voicing: the bells long and inharmonic, the xylophones short, the
/// cowbell clanky and short.
const INSTRUMENTS: [(&str, Family, f32, Voicing); 16] = [
    ("harp", Family::Pluck, 369.99, NO_VOICING),
    ("basedrum", Family::Membrane, 80.0, NO_VOICING),
    ("snare", Family::Membrane, 200.0, NO_VOICING),
    ("hat", Family::Membrane, 400.0, NO_VOICING),
    ("bass", Family::Pluck, 65.41, NO_VOICING),
    ("flute", Family::Breath, 440.0, NO_VOICING),
    (
        "bell",
        Family::Modal,
        659.26,
        Voicing {
            inharmonicity: 0.85,
            damping: 0.90,
        },
    ),
    ("guitar", Family::Pluck, 196.0, NO_VOICING),
    (
        "chime",
        Family::Modal,
        1318.51,
        Voicing {
            inharmonicity: 0.70,
            damping: 0.85,
        },
    ),
    (
        "xylophone",
        Family::Modal,
        1046.5,
        Voicing {
            inharmonicity: 0.10,
            damping: 0.25,
        },
    ),
    (
        "iron_xylophone",
        Family::Modal,
        1568.0,
        Voicing {
            inharmonicity: 0.50,
            damping: 0.70,
        },
    ),
    (
        "cow_bell",
        Family::Modal,
        659.26,
        Voicing {
            inharmonicity: 0.60,
            damping: 0.20,
        },
    ),
    ("didgeridoo", Family::Breath, 73.42, NO_VOICING),
    ("bit", Family::Electronic, 523.25, NO_VOICING),
    ("banjo", Family::Pluck, 392.0, NO_VOICING),
    ("pling", Family::Electronic, 440.0, NO_VOICING),
];

/// The family that models `instrument`, or `None` if it is not one of the
/// sixteen instruments Foley renders.
pub fn family(instrument: u32) -> Option<Family> {
    INSTRUMENTS.get(instrument as usize).map(|entry| entry.1)
}

/// The instrument's base frequency: the frequency (Hz) of its note 12.
pub fn base_hz(instrument: u32) -> Option<f32> {
    INSTRUMENTS.get(instrument as usize).map(|entry| entry.2)
}

/// The note's own frequency: the instrument's base times 2^((note-12)/12).
pub fn note_hz(instrument: u32, note: f32) -> Option<f32> {
    base_hz(instrument).map(|base| base * 2.0f32.powf((note - 12.0) / 12.0))
}

/// One note block hit, rendered at its own pitch. `character` (0 to 1)
/// varies the timbre so successive hits differ; the seed mixes the same.
pub fn note(instrument: u32, note: f32, character: f32, seed: u64) -> Option<Vec<f32>> {
    let family = family(instrument)?;
    let hz = note_hz(instrument, note)?;
    // A non-finite `character` would poison damping, brightness and the
    // loop gain with NaN; treat it as the neutral midpoint instead.
    let character = if character.is_finite() {
        character.clamp(0.0, 1.0)
    } else {
        0.5
    };
    // The seed mixes the character too, so two hits of the same note with
    // different characters never share a noise burst.
    let seed = seed ^ u64::from(character.to_bits());
    let mut out = match family {
        Family::Pluck => {
            // The four plucked strings, in the mapped index order: harp,
            // bass, guitar, banjo — clean, long and dull, between, bright
            // and snappy. `damping` sets how fast the string loses energy,
            // `brightness` how dull its feedback loop and its pluck are.
            let (damping, brightness) = match instrument {
                0 => (0.35, 0.65),  // harp: clean
                4 => (0.15, 0.20),  // bass: long and dull
                7 => (0.45, 0.45),  // guitar: between
                14 => (0.70, 0.90), // banjo: bright and snappy
                _ => (0.40, 0.50),
            };
            // `character` swings each hit's timbre around the voicing:
            // a low character damps more, so the hit is duller and dies
            // out sooner; a high character is plucked brighter and the
            // string rings longer.
            let damping = (damping * (1.3 - 0.6 * character)).clamp(0.0, 1.0);
            let brightness = (brightness + 0.2 * (character - 0.5)).clamp(0.0, 1.0);
            pluck(hz, damping, brightness, seed)
        }
        Family::Modal => {
            // The five struck instruments, in the mapped index order: bell,
            // chime, xylophone, iron_xylophone, cow_bell — the bells long and
            // inharmonic, the xylophone short and near-harmonic (a bar's
            // modes ≈ 1, 2.76, 5.40, 8.93), the iron bar metallic and long,
            // the cowbell clanky and short. `character` swings each hit
            // around the voicing: a low character strikes dull and dies
            // sooner — `damping` is the ring's length here, so it is
            // scaled *down* — while a high one rings on and sits further
            // off the series, the Pluck family's convention.
            let v = INSTRUMENTS[instrument as usize].3;
            let inharmonicity = (v.inharmonicity + 0.1 * (character - 0.5)).clamp(0.0, 1.0);
            let damping = (v.damping * (0.8 + 0.4 * character)).clamp(0.0, 1.0);
            modal(hz, inharmonicity, damping, seed)
        }
        Family::Membrane => return None,   // Task 3
        Family::Breath => return None,     // Task 4
        Family::Electronic => return None, // Task 5
    };
    crate::dsp::normalize_in_place(&mut out, 0.9);
    Some(out)
}

/// A plucked string (Karplus-Strong): a delay line of one period, seeded
/// with a short burst of lowpass-filtered noise — the pluck — fed back
/// through a one-pole lowpass, the string's damping of its own highs.
/// `damping` (0 to 1) is how readily the string loses energy: it sets both
/// the loop's lowpass coefficient and the loop gain, so a heavily damped
/// string rings for ~0.3 s while a light one rings on past 3 s.
/// `brightness` (0 to 1) cuts the pluck and the loop dull or bright, and
/// moves the pluck position with it: a bright banjo is struck near the
/// bridge, a dull bass over its middle.
fn pluck(hz: f32, damping: f32, brightness: f32, seed: u64) -> Vec<f32> {
    let hz = hz.max(20.0);
    let n = ((SAMPLE_RATE as f32 / hz).round() as usize).max(2);
    let mut rng = Rng::new(seed);
    // The pluck: noise lowpassed at a cutoff set by brightness.
    let mut lp = LowPass::new(250.0 + 7500.0 * brightness);
    let mut burst = Vec::with_capacity(n);
    for _ in 0..n {
        burst.push(lp.process(rng.signed()));
    }
    // Pluck position: notch the initial displacement the way a finger a
    // fraction along the string would — subtract the burst delayed by the
    // pluck point, killing the harmonics that have a node there.
    let pos = (((0.10 + 0.35 * (1.0 - brightness)) * n as f32) as usize)
        .clamp(1, n.saturating_sub(1).max(1));
    let mut line = vec![0.0f32; n];
    for (i, sample) in line.iter_mut().enumerate() {
        *sample = burst[i] - burst[(i + n - pos) % n];
    }
    // The loop: one-pole lowpass, then the gain that takes the string to
    // -60 dB over its decay time. -60 dB after t60 seconds is a round-trip
    // gain of 10^(-3 / (t60 * hz)) — one round trip per period.
    let damping = damping.clamp(0.0, 1.0);
    let t60 = 0.3 + 2.7 * (1.0 - damping);
    let gain = 10.0f32.powf(-3.0 / (t60 * hz));
    let max_len = (3.0 * SAMPLE_RATE as f32) as usize;
    let mut out = Vec::with_capacity(max_len.min((t60 * SAMPLE_RATE as f32) as usize + n));
    let mut idx = 0usize;
    let mut prev = 0.0f32;
    let mut peak = 0.0f32;
    let mut period_peak = 0.0f32;
    for i in 0..max_len {
        let x = line[idx];
        let s = gain * ((1.0 - damping) * x + damping * prev);
        prev = s;
        line[idx] = s;
        idx = (idx + 1) % n;
        out.push(s);
        peak = peak.max(s.abs());
        period_peak = period_peak.max(s.abs());
        // Stop once a full period stays below -60 dB of the loudest sample.
        if (i + 1) % n == 0 {
            if i + 1 >= 2 * n && period_peak <= 1e-3 * peak {
                break;
            }
            period_peak = 0.0;
        }
    }
    out
}

/// A struck bar or bell: a bank of decaying sinusoids. The modes start at a
/// free-free bar's ratios — 1, 2.76, 5.40, 8.93 — and spread upward off the
/// harmonic series as `inharmonicity` (0 to 1) rises, the way a metal bar's
/// modes sit high; the fundamental stays at `hz` whatever the material above
/// it does. `damping` (0 to 1) sets how long the strike rings — 0.1 dies
/// within a few hundred milliseconds, 0.9 rings on for seconds — and each
/// higher mode dies faster than the one below it. `seed` offsets the modes'
/// starting phases so two strikes of the same note differ.
fn modal(hz: f32, inharmonicity: f32, damping: f32, seed: u64) -> Vec<f32> {
    let hz = hz.max(20.0);
    let inharmonicity = inharmonicity.clamp(0.0, 1.0);
    let damping = damping.clamp(0.0, 1.0);
    // A free-free bar's mode ratios, how far each spreads upward as the
    // material grows more inharmonic, and how loudly each sounds at the
    // strike — louder, the lower the mode.
    let bar = [1.0f32, 2.76, 5.40, 8.93];
    let spread = [0.0f32, 1.0, 2.6, 5.0];
    let amps = [1.0f32, 0.35, 0.18, 0.09];
    // The ring: -60 dB after `t60` seconds, stretched by `damping`.
    let t60 = 0.15 + 3.0 * damping * damping;
    let modes = bar.len();
    // One decay per mode: -60 dB at t60 for the fundamental, a sixth of
    // that time for each mode above — higher partials die faster.
    let decay: Vec<f32> = (0..modes)
        .map(|i| (-3.0 * LN_10 / (t60 * 0.6f32.powi(i as i32) * SAMPLE_RATE as f32)).exp())
        .collect();
    // One frequency step per mode, spread off the harmonic series.
    let step: Vec<f32> = bar
        .iter()
        .zip(&spread)
        .map(|(b, s)| TAU * hz * (b + inharmonicity * s) / SAMPLE_RATE as f32)
        .collect();
    let mut rng = Rng::new(seed);
    let mut phase: Vec<f32> = (0..modes).map(|_| rng.unit() * TAU).collect();
    let mut level = amps;
    let length = (3.0 * SAMPLE_RATE as f32) as usize;
    // An 8 ms attack so the strike "swells" rather than clicks on.
    let attack = (0.008 * SAMPLE_RATE as f32) as usize;
    let mut out = Vec::with_capacity(length);
    for i in 0..length {
        let env = if i < attack {
            i as f32 / attack as f32
        } else {
            1.0
        };
        let mut s = 0.0f32;
        for m in 0..modes {
            s += level[m] * phase[m].sin();
            phase[m] += step[m];
        }
        out.push(env * s);
        for m in 0..modes {
            level[m] *= decay[m];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seventeen_instruments_map_to_the_expected_families() {
        assert_eq!(family(0), Some(Family::Pluck)); // harp
        assert_eq!(family(2), Some(Family::Membrane)); // snare
        assert_eq!(family(5), Some(Family::Breath)); // flute
        assert_eq!(family(6), Some(Family::Modal)); // bell
        assert_eq!(family(13), Some(Family::Electronic)); // bit
        assert_eq!(family(16), None); // out of range
    }

    #[test]
    fn a_note_octave_above_the_base_doubles_the_frequency() {
        let base = base_hz(0).unwrap();
        assert!((note_hz(0, 12.0).unwrap() - base).abs() < 1e-3);
        assert!((note_hz(0, 24.0).unwrap() - 2.0 * base).abs() < 1e-3);
        assert!((note_hz(0, 0.0).unwrap() - 0.5 * base).abs() < 1e-3);
    }

    #[test]
    fn an_unknown_instrument_renders_nothing() {
        assert!(note(16, 12.0, 0.5, 1).is_none());
    }

    #[test]
    fn a_plucked_note_renders_audible_finite_audio() {
        let out = note(0, 12.0, 0.5, 7);
        assert!(out.is_some(), "plucked note rendered nothing");
        let out = out.unwrap();
        assert!(!out.is_empty(), "plucked note is empty");
        assert!(
            out.iter().all(|x| x.is_finite()),
            "plucked note has non-finite samples"
        );
        let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(
            peak > 0.05 && peak <= 1.0,
            "peak {peak} not in (0.05, 1.0]: not audible, or clipping"
        );
        assert!(
            out.len() <= (3.0 * SAMPLE_RATE as f32) as usize,
            "plucked note runs past 3 s"
        );
        // The tail must be below -60 dB: a note that stops mid-cycle clicks.
        let tail = &out[out.len().saturating_sub(100)..];
        assert!(
            tail.iter().all(|x| x.abs() < 1e-3),
            "tail does not fade out — the note clicks off"
        );
        // An instrument past the table has no base and no pitch either.
        assert!(base_hz(16).is_none());
        assert!(note_hz(16, 12.0).is_none());
    }

    /// RMS of a signal.
    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt()
    }

    /// RMS of the back half: what is still ringing once the strike should
    /// have died away.
    fn tail_rms(s: &[f32]) -> f32 {
        rms(&s[s.len() / 2..])
    }

    /// The strike's fundamental, from the zero-crossing rate of the loudest
    /// window — the same helper shape `bell.rs` uses, widened to ~4096
    /// samples so one stray crossing cannot quantise the pitch by 60 Hz.
    /// The window is lowpassed first: a struck bar's modes would each add
    /// crossings of their own and inflate the rate far above the pitch the
    /// ear actually hears.
    fn fundamental(s: &[f32]) -> f32 {
        let loudest = s
            .iter()
            .enumerate()
            .fold(
                (0usize, 0f32),
                |m, (i, x)| if x.abs() > m.1 { (i, x.abs()) } else { m },
            )
            .0;
        let start = loudest.saturating_sub(2048);
        let end = (start + 4096).min(s.len());
        let mut lp = LowPass::new(600.0);
        let mut lp2 = LowPass::new(600.0);
        let mut lp3 = LowPass::new(600.0);
        let w: Vec<f32> = s[start..end]
            .iter()
            .map(|x| lp3.process(lp2.process(lp.process(*x))))
            .collect();
        let crossings = w.windows(2).filter(|x| x[0] <= 0.0 && x[1] > 0.0).count() as f32;
        crossings * SAMPLE_RATE as f32 / w.len().max(1) as f32
    }

    /// The strongest partial at or above `floor` Hz: a Goertzel scan of the
    /// strike's first quarter second, where every mode still sounds.
    fn strongest_partial(s: &[f32], floor: f32) -> f32 {
        use std::f32::consts::TAU;
        let w = &s[..s.len().min(SAMPLE_RATE as usize / 4)];
        let mut best_hz = 0.0f32;
        let mut best_mag = -1.0f32;
        let mut hz = floor;
        while hz < 6000.0 {
            let coeff = 2.0 * (TAU * hz / SAMPLE_RATE as f32).cos();
            let (mut s1, mut s2) = (0.0f32, 0.0f32);
            for &x in w {
                let s0 = x + coeff * s1 - s2;
                s2 = s1;
                s1 = s0;
            }
            let mag = (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(0.0).sqrt();
            if mag > best_mag {
                best_mag = mag;
                best_hz = hz;
            }
            hz += 10.0;
        }
        best_hz
    }

    /// How far the metal bar's strongest partial sits above the harmonic
    /// series compared with the near-harmonic bar's: the ratio of their
    /// strongest spectral peaks above the fundamental.
    fn partial_ratio(metal: &[f32], wood: &[f32]) -> f32 {
        let floor = 1.5 * fundamental(wood);
        strongest_partial(metal, floor) / strongest_partial(wood, floor)
    }

    #[test]
    fn a_bar_rings_at_the_note_it_was_struck_at() {
        let out = modal(440.0, 0.2, 0.5, 1);
        assert!(
            (fundamental(&out) - 440.0).abs() < 40.0,
            "bar at {}",
            fundamental(&out)
        );
    }

    #[test]
    fn an_inharmonic_bar_has_partials_off_the_harmonic_series() {
        // The 2nd partial of a metal bar is not 2x the fundamental.
        let wood = modal(440.0, 0.0, 0.5, 2);
        let metal = modal(440.0, 0.6, 0.5, 2);
        assert!(partial_ratio(&metal, &wood) > 1.1);
    }

    #[test]
    fn higher_damping_rings_longer() {
        let short = modal(440.0, 0.3, 0.1, 3);
        let long = modal(440.0, 0.3, 0.9, 3);
        assert!(tail_rms(&long) > 2.0 * tail_rms(&short));
    }

    #[test]
    fn a_struck_bar_renders_audible_finite_audio() {
        let out = note(6, 12.0, 0.5, 7);
        assert!(out.is_some(), "struck note rendered nothing");
        let out = out.unwrap();
        assert!(!out.is_empty(), "struck note is empty");
        assert!(
            out.iter().all(|x| x.is_finite()),
            "struck note has non-finite samples"
        );
        let peak = out.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(
            peak > 0.05 && peak <= 1.0,
            "peak {peak} not in (0.05, 1.0]: not audible, or clipping"
        );
        assert!(
            out.len() <= (3.0 * SAMPLE_RATE as f32) as usize,
            "struck note runs past 3 s"
        );
        // The tail must be below -60 dB: a note that stops mid-cycle clicks.
        let tail = &out[out.len().saturating_sub(100)..];
        assert!(
            tail.iter().all(|x| x.abs() < 1e-3),
            "tail does not fade out — the note clicks off"
        );
    }

    #[test]
    fn a_low_character_bell_dies_sooner_than_a_high_one() {
        // The Modal arm's character swing follows the Pluck family's
        // convention: a low character strikes dull and dies sooner, a
        // high one rings on.
        let dull = note(6, 12.0, 0.1, 9).unwrap();
        let bright = note(6, 12.0, 0.9, 9).unwrap();
        assert!(
            tail_rms(&dull) < 0.5 * tail_rms(&bright),
            "low character tail {} not clearly under high character tail {}",
            tail_rms(&dull),
            tail_rms(&bright)
        );
    }
}
