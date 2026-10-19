//! Creatures of other mods (F3): a voice from an anatomy — its tract's
//! length, its usual pitch, and how far it goes into roughness and breath —
//! making the five calls any creature needs: ambient, hurt, death, growl,
//! breath. Each is the same physics as Foley's own voices (an LF source with
//! nonlinear phenomena through a waveguide tract), made up afresh from the
//! seed, the archetypes keeping to the brief: unsettling in measure, never
//! outright horror.

use super::score::{AA, EH, OH, SCHWA, Seg, UH, Voice, c, finish_at, k, perform_at};
use crate::Rng;

/// Another mod's creature's voice. `length` is its vocal tract's (m) and
/// `f0` its usual pitch (Hz); formant spacing follows the tract (ΔF = c/2L),
/// and listeners read the body's size from both (Fitch 1997). `rough`
/// (0 to 1) is how far it goes into the nonlinear phenomena — chaos,
/// subharmonics, creak, the 30–150 Hz roughness of alarm — and `breathy` how
/// breathy it is, where 1 is as far as Foley's own go.
#[derive(Debug, Clone, Copy)]
pub struct CreatureSpec {
    pub length: f32,
    pub f0: f32,
    pub rough: f32,
    pub breathy: f32,
}

/// The calls any creature makes. A call a creature wasn't given is its
/// ambient one (the Java side maps the game's call names onto these).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum CreatureCall {
    Ambient,
    Hurt,
    Death,
    Growl,
    Breath,
}

/// A usual value wavered by the usual amount.
fn vary(rng: &mut Rng, base: f32, frac: f32) -> f32 {
    let gauss = rng.signed() + rng.signed() + rng.signed();
    base * (1.0 + frac * gauss / 3.0)
}

/// A creature of `spec` making `call`, made up from `seed`.
pub fn creature_voice(
    rng: &mut Rng,
    spec: &CreatureSpec,
    call: CreatureCall,
    sample_rate: u32,
) -> Vec<f32> {
    let segments = match call {
        CreatureCall::Ambient => ambient(rng, spec),
        CreatureCall::Hurt => hurt(rng, spec),
        CreatureCall::Death => death(rng, spec),
        CreatureCall::Growl => growl(rng, spec),
        CreatureCall::Breath => breath(rng, spec),
    };
    let voice = Voice {
        length: spec.length,
        f0: spec.f0 * (1.0 + 0.06 * rng.signed()),
    };
    let peak = match call {
        CreatureCall::Hurt => 0.65,
        CreatureCall::Death => 0.6,
        CreatureCall::Growl => 0.6,
        CreatureCall::Breath => 0.35,
        CreatureCall::Ambient => 0.5,
    };
    finish_at(
        perform_at(rng, voice, &segments, sample_rate),
        peak,
        sample_rate,
    )
}

/// An ambient call: a phrase or two, falling, in the creature's own quality.
fn ambient(rng: &mut Rng, spec: &CreatureSpec) -> Vec<Seg> {
    let mut segments = Vec::new();
    for _ in 0..1 + (rng.unit() * 2.0) as usize {
        let mut g =
            Seg::new(vary(rng, 1.1, 0.25)).vowels(&[(0.0, UH), (0.4, SCHWA), (0.8, OH), (1.0, UH)]);
        g.f0 = k(&[(0.0, 1.05), (0.5, 0.95), (1.0, 0.75)]);
        g.rd = c(1.6 + 0.6 * spec.breathy);
        g.loud = k(&[(0.0, 0.0), (0.2, 0.8), (0.5, 1.0), (0.85, 0.7), (1.0, 0.0)]);
        g.sub = c(0.35 * spec.rough);
        g.chaos = k(&[
            (0.0, 0.1 * spec.rough),
            (0.4, 0.35 * spec.rough),
            (1.0, 0.15 * spec.rough),
        ]);
        g.sidebands = c(0.15 * spec.rough);
        g.sideband_hz = 40.0 + 40.0 * rng.unit();
        g.breath = c(0.3 + 0.25 * spec.breathy);
        g.creak = k(&[
            (0.0, 0.4 * spec.rough),
            (0.15, 0.0),
            (0.75, 0.0),
            (1.0, 0.7 * spec.rough),
        ]);
        g.jitter = 0.02 * (1.0 + spec.rough);
        g.tremolo = (vary(rng, 4.5, 0.2), 0.1 + 0.1 * spec.rough);
        g.gap = vary(rng, 0.5, 0.3);
        segments.push(g);
    }
    segments
}

/// A hurt call: sharp and higher, with a burst of chaos, gone quickly.
fn hurt(rng: &mut Rng, spec: &CreatureSpec) -> Vec<Seg> {
    let mut cry = Seg::new(vary(rng, 0.35, 0.2)).vowels(&[(0.0, AA), (1.0, EH)]);
    cry.f0 = k(&[(0.0, 1.4), (0.25, 1.8), (1.0, 1.2)]);
    cry.rd = c(0.7);
    cry.loud = k(&[(0.0, 0.0), (0.06, 1.0), (0.5, 0.8), (1.0, 0.0)]);
    cry.chaos = k(&[
        (0.0, 0.5 * spec.rough),
        (0.5, 0.3 * spec.rough),
        (1.0, 0.05),
    ]);
    cry.sub = c(0.25 * spec.rough);
    cry.sidebands = c(0.2 + 0.2 * spec.rough);
    cry.sideband_hz = 60.0 + 50.0 * rng.unit();
    cry.breath = c(0.2 + 0.2 * spec.breathy);
    cry.jitter = 0.03 * (1.0 + spec.rough);
    vec![cry]
}

/// A death call: falling, fading, breathier, with a last ingressive catch.
fn death(rng: &mut Rng, spec: &CreatureSpec) -> Vec<Seg> {
    let mut d = Seg::new(vary(rng, 1.0, 0.2)).vowels(&[(0.0, AA), (0.6, UH), (1.0, SCHWA)]);
    d.f0 = k(&[(0.0, 1.3), (0.4, 0.9), (1.0, 0.5)]);
    d.rd = k(&[(0.0, 1.2), (1.0, 2.7)]);
    d.loud = k(&[(0.0, 0.0), (0.15, 0.9), (0.6, 0.5), (1.0, 0.0)]);
    d.chaos = k(&[
        (0.0, 0.3 * spec.rough),
        (0.6, 0.45 * spec.rough),
        (1.0, 0.2 * spec.rough),
    ]);
    d.sub = c(0.4 * spec.rough);
    d.breath = k(&[
        (0.0, 0.2 + 0.2 * spec.breathy),
        (1.0, 0.5 + 0.3 * spec.breathy),
    ]);
    d.creak = k(&[(0.5, 0.0), (1.0, 0.8 * spec.rough)]);
    d.gap = 0.0;
    let mut catch = Seg::new(vary(rng, 0.3, 0.2)).vowels(&[(0.0, SCHWA)]);
    catch.ingressive = true;
    catch.voicing = c(0.5);
    catch.breath = c(0.4 + 0.3 * spec.breathy);
    catch.f0 = c(1.1);
    catch.loud = k(&[(0.0, 0.4), (1.0, 0.0)]);
    vec![d, catch]
}

/// A growl: low and rumbling, out and in, sidebands at the roughness rates.
fn growl(rng: &mut Rng, spec: &CreatureSpec) -> Vec<Seg> {
    let mut g = Seg::new(vary(rng, 1.2, 0.2)).vowels(&[(0.0, UH), (1.0, AA)]);
    g.f0 = k(&[(0.0, 0.8), (0.5, 0.75), (1.0, 0.7)]);
    g.rd = c(0.5 + 0.4 * spec.breathy);
    g.loud = k(&[(0.0, 0.0), (0.2, 0.9), (0.7, 0.8), (1.0, 0.0)]);
    g.sub = c(0.45 + 0.25 * spec.rough);
    g.sidebands = c(0.2 + 0.3 * spec.rough);
    g.sideband_hz = 30.0 + 60.0 * spec.rough;
    g.chaos = k(&[
        (0.0, 0.15 * spec.rough),
        (0.5, 0.3 * spec.rough),
        (1.0, 0.2 * spec.rough),
    ]);
    g.breath = c(0.25 + 0.25 * spec.breathy);
    g.jitter = 0.03 * (1.0 + spec.rough);
    vec![g]
}

/// A breath or two, unvoiced, heavier the breathier the creature is.
fn breath(rng: &mut Rng, spec: &CreatureSpec) -> Vec<Seg> {
    let mut segments = Vec::new();
    for _ in 0..2 {
        let mut b = Seg::new(vary(rng, 0.6, 0.3)).vowels(&[(0.0, SCHWA)]);
        b.voicing = c(0.15);
        b.breath = c(0.5 + 0.4 * spec.breathy);
        b.f0 = c(0.9);
        b.fric = c(0.2 + 0.2 * spec.breathy);
        b.fric_at = (0.2, 0.6);
        b.loud = k(&[(0.0, 0.0), (0.3, 0.8), (1.0, 0.0)]);
        b.gap = vary(rng, 0.4, 0.4);
        segments.push(b);
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;

    fn crossings(s: &[f32]) -> f32 {
        s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f32 / s.len() as f32
    }

    #[test]
    fn every_call_of_every_kind_of_creature_renders() {
        for (what, spec) in [
            (
                "small bird",
                CreatureSpec {
                    length: 0.04,
                    f0: 1200.0,
                    rough: 0.2,
                    breathy: 0.4,
                },
            ),
            (
                "human-like",
                CreatureSpec {
                    length: 0.17,
                    f0: 120.0,
                    rough: 0.5,
                    breathy: 0.5,
                },
            ),
            (
                "huge hostile",
                CreatureSpec {
                    length: 0.7,
                    f0: 60.0,
                    rough: 1.0,
                    breathy: 0.3,
                },
            ),
            (
                "clean breathy",
                CreatureSpec {
                    length: 0.3,
                    f0: 300.0,
                    rough: 0.0,
                    breathy: 1.0,
                },
            ),
        ] {
            for call in [
                CreatureCall::Ambient,
                CreatureCall::Hurt,
                CreatureCall::Death,
                CreatureCall::Growl,
                CreatureCall::Breath,
            ] {
                let mut rng = Rng::new(7);
                let sound = creature_voice(&mut rng, &spec, call, SAMPLE_RATE);
                assert!(
                    !sound.is_empty()
                        && sound.len() < 3 * SAMPLE_RATE as usize
                        && sound.iter().all(|x| x.is_finite()),
                    "{what} {call:?}: {} samples",
                    sound.len()
                );
                let peak = sound.iter().fold(0f32, |p, x| p.max(x.abs()));
                assert!(peak > 0.3, "{what} {call:?} is silent: {peak}");
            }
        }
    }

    #[test]
    fn size_reads_from_the_tract_and_roughness_reads_as_roughness() {
        let mut rng = Rng::new(3);
        let big = creature_voice(
            &mut rng,
            &CreatureSpec {
                length: 0.6,
                f0: 70.0,
                rough: 0.5,
                breathy: 0.4,
            },
            CreatureCall::Ambient,
            SAMPLE_RATE,
        );
        let small = creature_voice(
            &mut rng,
            &CreatureSpec {
                length: 0.05,
                f0: 900.0,
                rough: 0.5,
                breathy: 0.4,
            },
            CreatureCall::Ambient,
            SAMPLE_RATE,
        );
        assert!(
            crossings(&small) > crossings(&big),
            "the bigger creature sounds lower"
        );
        let rough = creature_voice(
            &mut rng,
            &CreatureSpec {
                length: 0.17,
                f0: 120.0,
                rough: 1.0,
                breathy: 0.4,
            },
            CreatureCall::Growl,
            SAMPLE_RATE,
        );
        let clean = creature_voice(
            &mut rng,
            &CreatureSpec {
                length: 0.17,
                f0: 120.0,
                rough: 0.0,
                breathy: 0.4,
            },
            CreatureCall::Growl,
            SAMPLE_RATE,
        );
        // Roughness modulates at 30–150 Hz: more crossings through zero.
        assert!(
            crossings(&rough) > crossings(&clean),
            "rough reads as rough"
        );
    }

    #[test]
    fn a_hurt_call_is_higher_than_the_ambient_one() {
        let spec = CreatureSpec {
            length: 0.2,
            f0: 150.0,
            rough: 0.5,
            breathy: 0.5,
        };
        let mut rng = Rng::new(9);
        let hurt = creature_voice(&mut rng, &spec, CreatureCall::Hurt, SAMPLE_RATE);
        let ambient = creature_voice(&mut rng, &spec, CreatureCall::Ambient, SAMPLE_RATE);
        assert!(crossings(&hurt[..hurt.len() / 4]) > crossings(&ambient[..ambient.len() / 4]));
    }
}
