use std::f32::consts::{PI, TAU};

use crate::{LowPass, Rng, SAMPLE_RATE};

const SPEED_OF_SOUND: f32 = 343.0;

/// The mobs Foley can voice (M6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Mob {
    Cow,
    Pig,
    Sheep,
    Chicken,
    Villager,
    Zombie,
    Wolf,
    Cat,
}

impl Mob {
    pub const ALL: [Mob; 8] = [
        Mob::Cow,
        Mob::Pig,
        Mob::Sheep,
        Mob::Chicken,
        Mob::Villager,
        Mob::Zombie,
        Mob::Wolf,
        Mob::Cat,
    ];

    pub fn from_index(index: usize) -> Option<Mob> {
        Mob::ALL.get(index).copied()
    }

    /// Its vocal tract's length (m) and its voice's usual pitch (Hz). Formant
    /// spacing follows the tract (ΔF = c / 2L), which follows body size
    /// (Fitch 1997): a cow's 45 cm tract against a chicken's 7.
    fn anatomy(self) -> (f32, f32) {
        match self {
            Mob::Cow => (0.45, 120.0),
            Mob::Pig => (0.25, 150.0),
            Mob::Sheep => (0.25, 290.0),
            Mob::Chicken => (0.07, 380.0),
            Mob::Villager => (0.17, 125.0),
            Mob::Zombie => (0.17, 80.0),
            Mob::Wolf => (0.2, 450.0),
            Mob::Cat => (0.08, 600.0),
        }
    }
}

/// Which call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Call {
    Ambient,
    Hurt,
    Death,
}

impl Call {
    pub const ALL: [Call; 3] = [Call::Ambient, Call::Hurt, Call::Death];

    pub fn from_index(index: usize) -> Option<Call> {
        Call::ALL.get(index).copied()
    }
}

/// Vowels as scale factors on the neutral tube's first four formants (from
/// a 17 cm human tract's: [a] 730/1090/2440 Hz against 500/1500/2500, …).
const A: [f32; 4] = [1.46, 0.73, 0.98, 1.0];
const E: [f32; 4] = [1.06, 1.23, 1.0, 1.0];
const I: [f32; 4] = [0.54, 1.53, 1.2, 1.05];
const O: [f32; 4] = [0.9, 0.53, 0.95, 1.0];
const U: [f32; 4] = [0.6, 0.58, 0.9, 1.0];

/// A value over a stretch of time, by keyframes at fractions 0 to 1 of it.
#[derive(Debug, Clone)]
struct Keys<T: Copy>(Vec<(f32, T)>);

impl Keys<f32> {
    fn at(&self, t: f32) -> f32 {
        let keys = &self.0;
        let i = keys.iter().position(|k| k.0 > t).unwrap_or(keys.len());
        if i == 0 {
            return keys[0].1;
        }
        if i == keys.len() {
            return keys[keys.len() - 1].1;
        }
        let (t0, a) = keys[i - 1];
        let (t1, b) = keys[i];
        a + (b - a) * (t - t0) / (t1 - t0).max(1e-6)
    }
}

impl Keys<[f32; 4]> {
    fn at(&self, t: f32) -> [f32; 4] {
        let mut out = [0.0; 4];
        for (n, slot) in out.iter_mut().enumerate() {
            *slot = Keys(self.0.iter().map(|k| (k.0, k.1[n])).collect()).at(t);
        }
        out
    }
}

fn keys(points: &[(f32, f32)]) -> Keys<f32> {
    Keys(points.to_vec())
}

fn vowels(points: &[(f32, [f32; 4])]) -> Keys<[f32; 4]> {
    Keys(points.to_vec())
}

/// One stretch of sound: a syllable, a grunt, a breath.
#[derive(Debug, Clone)]
struct Segment {
    seconds: f32,
    /// Pitch, against the mob's usual.
    pitch: Keys<f32>,
    vowel: Keys<[f32; 4]>,
    /// 1 with the mouth shut (a hum's nasal murmur), 0 open.
    nasal: Keys<f32>,
    loud: Keys<f32>,
    /// Breath noise against the voice, 0 to 1 (1: a whisper or a pant).
    breath: f32,
    /// Roughness: every other glottal pulse weaker by this much (period
    /// doubling, as in a growl or a groan), 0 to 1.
    rough: f32,
    /// Cycle-to-cycle pitch wobble (jitter), as a fraction.
    jitter: f32,
    /// A bleat's tremolo: rate (Hz) and depth.
    tremolo: (f32, f32),
    /// Silence after it, seconds.
    gap: f32,
}

impl Segment {
    fn new(seconds: f32) -> Segment {
        Segment {
            seconds,
            pitch: keys(&[(0.0, 1.0)]),
            vowel: vowels(&[(0.0, A)]),
            nasal: keys(&[(0.0, 0.0)]),
            loud: keys(&[(0.0, 0.0), (0.1, 1.0), (0.8, 0.9), (1.0, 0.0)]),
            breath: 0.1,
            rough: 0.0,
            jitter: 0.01,
            tremolo: (0.0, 0.0),
            gap: 0.0,
        }
    }
}

/// A formant: a two-pole resonator with unity gain at 0 Hz (Klatt's), so a
/// cascade of them gives a vowel's natural spectrum.
#[derive(Debug, Clone, Default)]
struct Resonator {
    a: f32,
    b: f32,
    c: f32,
    y1: f32,
    y2: f32,
}

impl Resonator {
    fn tune(&mut self, frequency: f32, bandwidth: f32) {
        let rate = SAMPLE_RATE as f32;
        let f = frequency.clamp(50.0, 0.45 * rate);
        let r = (-PI * bandwidth / rate).exp();
        self.c = -r * r;
        self.b = 2.0 * r * (TAU * f / rate).cos();
        self.a = 1.0 - self.b - self.c;
    }

    fn process(&mut self, x: f32) -> f32 {
        let y = self.a * x + self.b * self.y1 + self.c * self.y2;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Sings `segments` with a tract `length` m long and usual pitch `pitch`
/// Hz: Rosenberg glottal pulses (opening over 40 % of the period, closing
/// over 16 %), with jitter, shimmer, roughness and breath noise; radiated
/// from the lips (differentiated); through a cascade of four formants at
/// F_n = (2n − 1)·c / 4L times the vowel's factors, bandwidths a tenth of
/// each (at least 60 Hz); or, mouth shut, as a nasal murmur near 250 Hz.
fn sing(rng: &mut Rng, segments: &[Segment], length: f32, pitch: f32) -> Vec<f32> {
    let rate = SAMPLE_RATE as f32;
    let neutral: [f32; 4] =
        std::array::from_fn(|n| (2 * n + 1) as f32 * SPEED_OF_SOUND / (4.0 * length));
    let mut tract: [Resonator; 4] = Default::default();
    let mut murmur = Resonator::default();
    murmur.tune(250.0 * (0.17 / length).sqrt(), 100.0);
    let mut murmur_low = LowPass::new(500.0);
    let mut out = Vec::new();
    let (mut phase, mut previous, mut period_scale, mut period_gain, mut odd) =
        (0.0f32, 0.0f32, 1.0f32, 1.0f32, false);
    for segment in segments {
        let samples = (segment.seconds * rate) as usize;
        for i in 0..samples {
            let t = i as f32 / samples as f32;
            let mut f0 = pitch * segment.pitch.at(t) * period_scale;
            if segment.tremolo.0 > 0.0 {
                f0 *= 1.0
                    + 0.06 * segment.tremolo.1 * (TAU * segment.tremolo.0 * i as f32 / rate).sin();
            }
            phase += f0 / rate;
            if phase >= 1.0 {
                phase -= 1.0;
                // A new glottal cycle: jitter and shimmer, and roughness on
                // every other one.
                period_scale = 1.0 + segment.jitter * rng.signed() * 1.7;
                period_gain = 1.0 + 0.1 * rng.signed();
                odd = !odd;
                if odd {
                    period_gain *= 1.0 - segment.rough;
                }
            }
            let glottis = if phase < 0.4 {
                0.5 * (1.0 - (PI * phase / 0.4).cos())
            } else if phase < 0.56 {
                (PI * (phase - 0.4) / 0.32).cos()
            } else {
                0.0
            };
            // The lips radiate the flow's change.
            let voice = (glottis - previous) * rate / f0.max(1.0) * period_gain;
            previous = glottis;
            let noise = rng.signed() * (0.3 + 0.7 * glottis);
            let source = (1.0 - segment.breath) * voice + segment.breath * 2.0 * noise;
            let vowel = segment.vowel.at(t);
            for (n, resonator) in tract.iter_mut().enumerate() {
                let f = neutral[n] * vowel[n];
                resonator.tune(f, (0.1 * f).max(60.0));
            }
            let mut open = source;
            for resonator in &mut tract {
                open = resonator.process(open);
            }
            let shut = murmur_low.process(murmur.process(source));
            let nasal = segment.nasal.at(t).clamp(0.0, 1.0);
            let mut loud = segment.loud.at(t);
            if segment.tremolo.0 > 0.0 {
                loud *= 1.0
                    - segment.tremolo.1
                        * 0.5
                        * (1.0 + (TAU * segment.tremolo.0 * i as f32 / rate).sin());
            }
            out.push(loud * ((1.0 - nasal) * open + nasal * 3.0 * shut));
        }
        out.extend(std::iter::repeat_n(0.0, (segment.gap * rate) as usize));
    }
    out
}

/// What `mob` says for `call`: a few segments, made up afresh from `rng`
/// each time.
fn script(rng: &mut Rng, mob: Mob, call: Call) -> Vec<Segment> {
    let vary = |rng: &mut Rng, x: f32, by: f32| x * (1.0 + by * rng.signed());
    match call {
        Call::Hurt => {
            let mut s = Segment::new(vary(rng, 0.25, 0.2));
            s.pitch = keys(&[(0.0, 1.4), (0.3, 1.8), (1.0, 1.2)]);
            s.loud = keys(&[(0.0, 0.0), (0.05, 1.0), (0.6, 0.8), (1.0, 0.0)]);
            s.rough = 0.4;
            s.breath = 0.3;
            s.jitter = 0.03;
            if mob == Mob::Villager {
                s.vowel = vowels(&[(0.0, U), (1.0, O)]);
            }
            vec![s]
        }
        Call::Death => {
            let mut s = Segment::new(vary(rng, 0.9, 0.15));
            s.pitch = keys(&[(0.0, 1.3), (1.0, 0.55)]);
            s.vowel = vowels(&[(0.0, A), (1.0, U)]);
            s.loud = keys(&[(0.0, 0.0), (0.05, 1.0), (1.0, 0.0)]);
            s.rough = 0.3;
            s.breath = 0.35;
            s.jitter = 0.03;
            vec![s]
        }
        Call::Ambient => match mob {
            Mob::Cow => {
                // "Mm-ooo": a hum that opens into a low vowel.
                let mut s = Segment::new(vary(rng, 1.4, 0.2));
                s.pitch = keys(&[(0.0, 0.9), (0.3, 1.25), (0.8, 1.1), (1.0, 0.8)]);
                s.nasal = keys(&[(0.0, 1.0), (0.25, 0.9), (0.4, 0.1), (1.0, 0.2)]);
                s.vowel = vowels(&[(0.0, U), (0.4, O), (0.8, U)]);
                s.loud = keys(&[(0.0, 0.0), (0.1, 0.6), (0.4, 1.0), (0.85, 0.8), (1.0, 0.0)]);
                s.breath = 0.15;
                s.rough = 0.05;
                vec![s]
            }
            Mob::Pig => (0..2 + (rng.unit() * 2.0) as usize)
                .map(|_| {
                    // Short, rough, nasal grunts.
                    let mut s = Segment::new(vary(rng, 0.14, 0.3));
                    s.pitch = keys(&[(0.0, 1.1), (1.0, 0.8)]);
                    s.nasal = keys(&[(0.0, 0.6)]);
                    s.vowel = vowels(&[(0.0, O), (1.0, U)]);
                    s.rough = 0.35;
                    s.breath = 0.4;
                    s.gap = vary(rng, 0.08, 0.4);
                    s
                })
                .collect(),
            Mob::Sheep => {
                // "Baa", with a bleat's tremolo.
                let mut s = Segment::new(vary(rng, 0.9, 0.2));
                s.pitch = keys(&[(0.0, 0.95), (0.2, 1.1), (1.0, 0.85)]);
                s.vowel = vowels(&[(0.0, E), (0.2, A)]);
                s.tremolo = (vary(rng, 8.0, 0.15), 0.5);
                s.breath = 0.25;
                s.rough = 0.1;
                vec![s]
            }
            Mob::Chicken => {
                let mut clucks: Vec<Segment> = (0..3 + (rng.unit() * 3.0) as usize)
                    .map(|_| {
                        let mut s = Segment::new(vary(rng, 0.07, 0.2));
                        s.pitch = keys(&[(0.0, 1.0), (1.0, 0.87)]);
                        s.vowel = vowels(&[(0.0, O), (1.0, A)]);
                        s.loud = keys(&[(0.0, 0.0), (0.15, 1.0), (1.0, 0.0)]);
                        s.rough = 0.2;
                        s.breath = 0.2;
                        s.gap = vary(rng, 0.12, 0.3);
                        s
                    })
                    .collect();
                if rng.unit() < 0.4 {
                    // "Bawk!"
                    let mut s = Segment::new(vary(rng, 0.25, 0.2));
                    s.pitch = keys(&[(0.0, 1.2), (0.4, 1.37), (1.0, 1.05)]);
                    s.vowel = vowels(&[(0.0, O), (0.3, A)]);
                    s.rough = 0.25;
                    clucks.push(s);
                }
                clucks
            }
            Mob::Villager => {
                // "Hmm", "hrm?", "hm.": a hum, mouth shut, in one of three tunes.
                let mut s = Segment::new(vary(rng, 0.5, 0.35));
                s.nasal = keys(&[(0.0, 1.0)]);
                s.pitch = match (rng.unit() * 3.0) as usize {
                    0 => keys(&[(0.0, 1.12), (1.0, 0.84)]),
                    1 => keys(&[(0.0, 0.88), (1.0, 1.2)]),
                    _ => keys(&[(0.0, 0.92), (0.5, 1.16), (1.0, 0.88)]),
                };
                s.breath = 0.05;
                vec![s]
            }
            Mob::Zombie => {
                // A rough, breathy groan, "uh-aah-oh".
                let mut s = Segment::new(vary(rng, 1.2, 0.3));
                s.pitch = keys(&[(0.0, 1.06), (0.5, 0.94), (1.0, 0.75)]);
                s.vowel = vowels(&[(0.0, U), (0.3, A), (0.8, O), (1.0, U)]);
                s.nasal = keys(&[(0.0, 0.1)]);
                s.rough = 0.5;
                s.jitter = 0.04;
                s.breath = 0.35;
                vec![s]
            }
            Mob::Wolf => {
                if rng.unit() < 0.7 {
                    // Panting: breaths through an open mouth, about three a second.
                    (0..4 + (rng.unit() * 3.0) as usize)
                        .map(|k| {
                            let mut s = Segment::new(vary(rng, 0.12, 0.2));
                            s.breath = 1.0;
                            s.vowel = vowels(&[(0.0, if k % 2 == 0 { A } else { E })]);
                            s.loud = keys(&[(0.0, 0.0), (0.3, 0.6), (1.0, 0.0)]);
                            s.gap = vary(rng, 0.2, 0.2);
                            s
                        })
                        .collect()
                } else {
                    // A bark or two.
                    (0..1 + (rng.unit() * 2.0) as usize)
                        .map(|_| {
                            let mut s = Segment::new(vary(rng, 0.12, 0.2));
                            s.pitch = keys(&[(0.0, 1.1), (1.0, 0.75)]);
                            s.vowel = vowels(&[(0.0, A), (1.0, U)]);
                            s.loud = keys(&[(0.0, 0.0), (0.08, 1.0), (1.0, 0.0)]);
                            s.rough = 0.4;
                            s.breath = 0.5;
                            s.gap = vary(rng, 0.25, 0.3);
                            s
                        })
                        .collect()
                }
            }
            Mob::Cat => {
                // "Mi-a-ow".
                let mut s = Segment::new(vary(rng, 0.7, 0.25));
                s.pitch = keys(&[(0.0, 0.92), (0.35, 1.33), (1.0, 0.83)]);
                s.vowel = vowels(&[(0.0, I), (0.3, A), (0.8, U)]);
                s.nasal = keys(&[(0.0, 0.8), (0.12, 0.0)]);
                vec![s]
            }
        },
    }
}

/// A call of `mob` (M6), a source–filter voice from its anatomy, made up
/// from `seed`.
pub fn voice(mob: Mob, call: Call, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let (length, pitch) = mob.anatomy();
    let segments = script(&mut rng, mob, call);
    let pitch = pitch * (1.0 + 0.06 * rng.signed());
    let mut out = sing(&mut rng, &segments, length, pitch);
    out.extend(std::iter::repeat_n(0.0, SAMPLE_RATE as usize / 50));
    let loudest = out.iter().fold(0f32, |p, x| p.max(x.abs()));
    if loudest > 0.0 {
        for sample in &mut out {
            *sample *= 0.6 / loudest;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crossings(s: &[f32]) -> f32 {
        s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f32 / s.len() as f32
    }

    #[test]
    fn every_mob_has_every_call() {
        for mob in Mob::ALL {
            for call in Call::ALL {
                let s = voice(mob, call, 3);
                assert!(s.iter().all(|x| x.is_finite()), "{mob:?} {call:?}");
                assert!(
                    s.len() > SAMPLE_RATE as usize / 10 && s.len() < 3 * SAMPLE_RATE as usize,
                    "{mob:?} {call:?}"
                );
                assert!(s.iter().fold(0f32, |p, x| p.max(x.abs())) > 0.5);
            }
        }
    }

    #[test]
    fn a_cow_is_deeper_than_a_cat() {
        let cow = voice(Mob::Cow, Call::Ambient, 1);
        let cat = voice(Mob::Cat, Call::Ambient, 1);
        assert!(crossings(&cow) < crossings(&cat));
    }

    #[test]
    fn formants_follow_the_tract() {
        // A 17 cm tract's neutral formants: 504, 1513, 2522 Hz.
        let f1 = SPEED_OF_SOUND / (4.0 * 0.17);
        assert!((f1 - 504.0).abs() < 2.0);
    }
}
