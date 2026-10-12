//! Speech-like and other patterned vocalizations: pseudo-speech babble
//! (consonant–vowel syllables with phrase intonation, no words), laughter,
//! and sniffing.

use super::score::{AA, AE, EE, EH, OH, OO, SCHWA, Seg, UH, Vowel, c, k};
use crate::Rng;

/// How a voice babbles.
#[derive(Debug, Clone, Copy)]
pub(super) struct Babble {
    pub syllables: usize,
    /// Syllables a second.
    pub rate: f32,
    /// Pitch at the phrase's start and end, against the voice's usual.
    pub from: f32,
    pub to: f32,
    pub rd: f32,
    pub breath: f32,
    pub nasal: f32,
    pub chaos: f32,
    pub ingressive: bool,
    /// Fry on the last syllable.
    pub creak_end: bool,
    /// How open the lips are kept (a mumble is less).
    pub lips: f32,
}

#[derive(Debug, Clone, Copy)]
enum Consonant {
    /// A stop: a closure at a place (0 glottis to 1 lips), then release.
    Stop(f32),
    Nasal(f32),
    /// A fricative at a place, through a gap this wide (cm).
    Fricative(f32, f32),
    Breath,
    None,
}

const LIPS: f32 = 0.97;
const TEETH: f32 = 0.8;
const VELUM: f32 = 0.6;

fn consonant(rng: &mut Rng) -> Consonant {
    let place = [LIPS, TEETH, VELUM][(rng.unit() * 3.0) as usize % 3];
    match (rng.unit() * 100.0) as u32 {
        0..35 => Consonant::Stop(place),
        35..55 => Consonant::Nasal(if place == VELUM { TEETH } else { place }),
        55..75 => Consonant::Fricative(if place == LIPS { 0.92 } else { 0.85 }, 0.18),
        75..85 => Consonant::Breath,
        _ => Consonant::None,
    }
}

fn vowel(rng: &mut Rng) -> Vowel {
    [AA, AE, EH, EE, UH, SCHWA, OH, OO][(rng.unit() * 8.0) as usize % 8]
}

/// Pseudo-speech: syllables of a consonant and a vowel, the consonant made
/// in the tract (a closure released, a nasal, turbulence at a narrowing, a
/// breath), the pitch drifting from `from` to `to` over the phrase with a
/// stress here and there, and pauses between "words".
pub(super) fn babble(b: Babble, rng: &mut Rng) -> Vec<Seg> {
    let mut out = Vec::new();
    let period = 1.0 / b.rate;
    let mut word = 0;
    for i in 0..b.syllables {
        let x = i as f32 / b.syllables.max(1) as f32;
        let stress = if rng.unit() < 0.3 { 1.12 } else { 1.0 };
        let pitch = (b.from + (b.to - b.from) * x) * stress * (1.0 + 0.03 * rng.signed());
        let v = vowel(rng);
        let held = period * (0.75 + 0.5 * rng.unit());
        let lead = (0.04 + 0.03 * rng.unit()).min(0.4 * held);
        let mut cons = Seg::new(lead)
            .vowels(&[(0.0, v)])
            .f0(&[(0.0, pitch)])
            .rd(b.rd);
        cons.loud = c(0.35);
        cons.ingressive = b.ingressive;
        cons.lips = c(b.lips);
        match consonant(rng) {
            Consonant::Stop(place) => {
                cons = cons.fric(0.05, place, 0.0).voicing(0.3);
            }
            Consonant::Nasal(place) => {
                cons = cons.fric(0.03, place, 0.0).velum(&[(0.0, 1.0)]);
                cons.loud = c(0.6);
            }
            Consonant::Fricative(place, width) => {
                cons = cons.fric(0.5, place, width).voicing(0.2);
            }
            Consonant::Breath => {
                cons = cons.voicing(0.0).breath(0.6);
            }
            Consonant::None => {}
        }
        let mut vow = Seg::new(held - lead)
            .vowels(&[(0.0, v), (1.0, vowel(rng))])
            .f0(&[(0.0, pitch), (1.0, pitch * 0.97)])
            .rd(b.rd)
            .breath(b.breath)
            .chaos(b.chaos)
            .loud(&[(0.0, 0.6), (0.2, 1.0), (0.8, 0.8), (1.0, 0.35)]);
        vow.velum = c(b.nasal);
        vow.lips = c(b.lips);
        vow.ingressive = b.ingressive;
        if b.creak_end && i + 1 == b.syllables {
            vow = vow.creak(&[(0.0, 0.0), (0.4, 0.6), (1.0, 0.9)]);
            vow.loud = k(&[(0.0, 0.6), (0.2, 1.0), (1.0, 0.0)]);
        }
        word += 1;
        if word >= 2 + (rng.unit() * 3.0) as usize && i + 1 < b.syllables {
            vow.gap = 0.08 + 0.15 * rng.unit();
            vow.loud = k(&[(0.0, 0.6), (0.2, 1.0), (1.0, 0.0)]);
            word = 0;
        }
        out.push(cons);
        out.push(vow);
    }
    if let Some(last) = out.last_mut() {
        last.loud = k(&[(0.0, 0.6), (0.2, 1.0), (1.0, 0.0)]);
    }
    out
}

/// Laughter: `bursts` of "ha"/"heh"/"hee", each a breath then a voiced
/// vowel, about five a second, falling in pitch from `from` to `to`, with an
/// in-breath gasp at the end if `gasp`.
pub(super) fn laugh(
    bursts: usize,
    from: f32,
    to: f32,
    rd: f32,
    sound: Vowel,
    gasp: bool,
    rng: &mut Rng,
) -> Vec<Seg> {
    let mut out = Vec::new();
    for i in 0..bursts {
        let x = i as f32 / bursts.max(1) as f32;
        let pitch = from + (to - from) * x;
        let h = Seg::new(0.025)
            .vowels(&[(0.0, sound)])
            .voicing(0.0)
            .breath(0.7)
            .loud(&[(0.0, 0.3), (1.0, 0.8)]);
        let v = Seg::new(0.07 + 0.04 * rng.unit())
            .vowels(&[(0.0, sound)])
            .f0(&[(0.0, pitch * 1.05), (0.4, pitch * 1.1), (1.0, pitch * 0.92)])
            .rd(rd)
            .breath(0.35)
            .loud(&[(0.0, 0.8), (0.3, 1.0), (1.0, 0.0)])
            .gap(0.04 + 0.04 * rng.unit());
        out.push(h);
        out.push(v);
    }
    if gasp {
        out.push(
            Seg::new(0.25)
                .vowels(&[(0.0, UH)])
                .inward()
                .voicing(0.4)
                .breath(0.6)
                .f0(&[(0.0, from * 1.3)])
                .loud(&[(0.0, 0.0), (0.3, 0.7), (1.0, 0.0)]),
        );
    }
    out
}

/// Sniffing or snorting: short breaths through the nose, in (and sometimes
/// out), unvoiced, `rate` a second; `flutter` vibrates the nostrils (a
/// horse's snort).
pub(super) fn sniff(count: usize, rate: f32, flutter: f32, rng: &mut Rng) -> Vec<Seg> {
    (0..count)
        .map(|i| {
            let mut s = Seg::new(0.5 / rate * (0.8 + 0.4 * rng.unit()))
                .vowels(&[(0.0, SCHWA)])
                .nasal()
                .voicing(0.0)
                .breath(0.9)
                .loud(&[(0.0, 0.0), (0.3, 1.0), (1.0, 0.0)])
                .gap(0.5 / rate * (0.8 + 0.4 * rng.unit()));
            s.ingressive = i % 2 == 0;
            if flutter > 0.0 {
                s = s
                    .sidebands(flutter, 25.0 + 10.0 * rng.unit())
                    .fric(0.3, 0.97, 0.3);
                s.lips = c(0.3);
            }
            s
        })
        .collect()
}

/// A short, unvoiced vowel-coloured exhale. Used for hisses and huffs.
pub(super) fn huff(secs: f32, v: Vowel, loud: f32) -> Seg {
    let mut s = Seg::new(secs).vowels(&[(0.0, v)]).voicing(0.0).breath(0.8);
    s.loud = k(&[(0.0, 0.0), (0.15, loud), (1.0, 0.0)]);
    s
}
