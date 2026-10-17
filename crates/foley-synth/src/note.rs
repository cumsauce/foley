//! One note block hit: the instrument table mapping a vanilla instrument
//! index to the family that models it and the base frequency (Hz) of its
//! note 12, the pitch law, and — as this plan lands — one synthesiser per
//! family. The index order is Foley's own; the Java twin is
//! `NoteBlocks.INSTRUMENTS`.

use crate::LowPass;
use crate::Rng;
use crate::SAMPLE_RATE;

/// Which family models an instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Pluck,
    Modal,
    Membrane,
    Breath,
    Electronic,
}

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
/// later tasks of this plan.
const INSTRUMENTS: [(&str, Family, f32); 16] = [
    ("harp", Family::Pluck, 369.99),
    ("basedrum", Family::Membrane, 80.0),
    ("snare", Family::Membrane, 200.0),
    ("hat", Family::Membrane, 400.0),
    ("bass", Family::Pluck, 65.41),
    ("flute", Family::Breath, 440.0),
    ("bell", Family::Modal, 659.26),
    ("guitar", Family::Pluck, 196.0),
    ("chime", Family::Modal, 1318.51),
    ("xylophone", Family::Modal, 1046.5),
    ("iron_xylophone", Family::Modal, 1568.0),
    ("cow_bell", Family::Modal, 659.26),
    ("didgeridoo", Family::Breath, 73.42),
    ("bit", Family::Electronic, 523.25),
    ("banjo", Family::Pluck, 392.0),
    ("pling", Family::Electronic, 440.0),
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
        Family::Modal => return None,      // Task 2
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
}
