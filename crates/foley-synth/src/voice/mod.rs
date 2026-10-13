//! Mob voices (M6, remade in M7): a Liljencrants–Fant glottal source with
//! nonlinear phenomena (subharmonics, sidebands, chaos, pitch jumps, creak)
//! and ingressive voicing, through a Kelly–Lochbaum waveguide tract with a
//! nasal branch and frication, sized by each mob's anatomy; and, for mobs with
//! no voice, what they are made of. docs/PLAN.md has the research behind it.

mod creatures;
mod lf;
mod mobs2;
mod score;
mod speech;
mod tract;

use creatures::{Bones, Call3, CreeperCall};
use score::{AA, AE, EE, EH, K, OH, OO, Seg, UH, Voice, c, finish, k, perform};

use crate::Rng;

pub use lf::{Pulse, lf_pulse};

/// The mobs Foley can voice. The order is the C ABI's.
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
    Husk,
    Drowned,
    ZombieVillager,
    Skeleton,
    Stray,
    Bogged,
    Spider,
    CaveSpider,
    Creeper,
    Bat,
    Horse,
    Donkey,
    Mule,
    Goat,
    Llama,
    Rabbit,
    Fox,
    Parrot,
    Bee,
    PolarBear,
    Panda,
    Frog,
    Camel,
    Armadillo,
    Turtle,
    Ocelot,
    WanderingTrader,
    IronGolem,
    SnowGolem,
    Enderman,
    Witch,
    Slime,
    SlimeSmall,
    Phantom,
    Pillager,
    Vindicator,
    Evoker,
    Illusioner,
    Ravager,
    Vex,
    Guardian,
    ElderGuardian,
    Silverfish,
    Endermite,
}

impl Mob {
    pub const ALL: [Mob; 52] = [
        Mob::Cow,
        Mob::Pig,
        Mob::Sheep,
        Mob::Chicken,
        Mob::Villager,
        Mob::Zombie,
        Mob::Wolf,
        Mob::Cat,
        Mob::Husk,
        Mob::Drowned,
        Mob::ZombieVillager,
        Mob::Skeleton,
        Mob::Stray,
        Mob::Bogged,
        Mob::Spider,
        Mob::CaveSpider,
        Mob::Creeper,
        Mob::Bat,
        Mob::Horse,
        Mob::Donkey,
        Mob::Mule,
        Mob::Goat,
        Mob::Llama,
        Mob::Rabbit,
        Mob::Fox,
        Mob::Parrot,
        Mob::Bee,
        Mob::PolarBear,
        Mob::Panda,
        Mob::Frog,
        Mob::Camel,
        Mob::Armadillo,
        Mob::Turtle,
        Mob::Ocelot,
        Mob::WanderingTrader,
        Mob::IronGolem,
        Mob::SnowGolem,
        Mob::Enderman,
        Mob::Witch,
        Mob::Slime,
        Mob::SlimeSmall,
        Mob::Phantom,
        Mob::Pillager,
        Mob::Vindicator,
        Mob::Evoker,
        Mob::Illusioner,
        Mob::Ravager,
        Mob::Vex,
        Mob::Guardian,
        Mob::ElderGuardian,
        Mob::Silverfish,
        Mob::Endermite,
    ];

    pub fn from_index(index: usize) -> Option<Mob> {
        Mob::ALL.get(index).copied()
    }

    /// Its vocal tract's length (m) and usual pitch (Hz). Formant spacing
    /// follows the tract (ΔF = c / 2L), and both follow body size (Fitch
    /// 1997).
    fn voice(self) -> Voice {
        let (length, f0) = match self {
            Mob::Cow => (0.45, 115.0),
            Mob::Pig => (0.25, 140.0),
            Mob::Sheep => (0.25, 280.0),
            Mob::Chicken => (0.07, 350.0),
            Mob::Villager => (0.17, 120.0),
            Mob::Zombie => (0.17, 85.0),
            Mob::Husk => (0.17, 75.0),
            Mob::Drowned => (0.17, 80.0),
            Mob::ZombieVillager => (0.17, 95.0),
            Mob::Wolf => (0.2, 420.0),
            Mob::Cat => (0.08, 600.0),
            Mob::Horse => (0.6, 500.0),
            Mob::Donkey => (0.45, 200.0),
            Mob::Mule => (0.5, 380.0),
            Mob::Goat => (0.25, 380.0),
            Mob::Llama => (0.4, 200.0),
            Mob::Rabbit => (0.05, 1200.0),
            Mob::Fox => (0.12, 700.0),
            Mob::Parrot => (0.04, 1100.0),
            Mob::PolarBear => (0.35, 90.0),
            Mob::Panda => (0.3, 300.0),
            Mob::Frog => (0.04, 1000.0),
            Mob::Camel => (0.6, 90.0),
            Mob::Armadillo => (0.04, 700.0),
            Mob::Turtle => (0.06, 200.0),
            Mob::Ocelot => (0.07, 750.0),
            Mob::WanderingTrader => (0.17, 105.0),
            Mob::Enderman => (0.22, 70.0),
            Mob::Witch => (0.16, 280.0),
            Mob::Phantom => (0.08, 1600.0),
            Mob::Pillager => (0.17, 105.0),
            Mob::Vindicator => (0.17, 95.0),
            Mob::Evoker => (0.17, 90.0),
            Mob::Illusioner => (0.17, 110.0),
            Mob::Ravager => (0.7, 65.0),
            Mob::Vex => (0.05, 1500.0),
            Mob::Guardian => (0.3, 220.0),
            Mob::ElderGuardian => (0.6, 110.0),
            _ => (0.17, 100.0),
        };
        Voice { length, f0 }
    }
}

/// Which call. The order is the C ABI's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Call {
    Ambient,
    Hurt,
    Death,
    Growl,
    Pant,
    Whine,
    Hiss,
    Purr,
    Beg,
    Primed,
    AmbientWater,
    HurtWater,
    DeathWater,
    /// Breathing, close by: steady, then heavy and fast.
    Breath,
    BreathHeavy,
    /// The ambient call close to the listener: more effort, louder.
    AmbientNear,
    Angry,
    Snort,
    Scream,
    Screech,
    Warning,
    Sneeze,
    Worried,
    Aggressive,
    Yes,
    No,
    Trade,
    Celebrate,
    Stare,
    Squish,
    Jump,
    Attack,
    Roar,
    Stunned,
    Charge,
    Swoop,
    Flap,
    Flop,
    AmbientLand,
    Loop,
    LoopAggressive,
    Sleep,
    AmbientBaby,
}

impl Call {
    pub const ALL: [Call; 43] = [
        Call::Ambient,
        Call::Hurt,
        Call::Death,
        Call::Growl,
        Call::Pant,
        Call::Whine,
        Call::Hiss,
        Call::Purr,
        Call::Beg,
        Call::Primed,
        Call::AmbientWater,
        Call::HurtWater,
        Call::DeathWater,
        Call::Breath,
        Call::BreathHeavy,
        Call::AmbientNear,
        Call::Angry,
        Call::Snort,
        Call::Scream,
        Call::Screech,
        Call::Warning,
        Call::Sneeze,
        Call::Worried,
        Call::Aggressive,
        Call::Yes,
        Call::No,
        Call::Trade,
        Call::Celebrate,
        Call::Stare,
        Call::Squish,
        Call::Jump,
        Call::Attack,
        Call::Roar,
        Call::Stunned,
        Call::Charge,
        Call::Swoop,
        Call::Flap,
        Call::Flop,
        Call::AmbientLand,
        Call::Loop,
        Call::LoopAggressive,
        Call::Sleep,
        Call::AmbientBaby,
    ];

    pub fn from_index(index: usize) -> Option<Call> {
        Call::ALL.get(index).copied()
    }

    /// The same call out of water, and whether it was in water.
    fn dry(self) -> (Call, bool) {
        match self {
            Call::AmbientWater => (Call::Ambient, true),
            Call::HurtWater => (Call::Hurt, true),
            Call::DeathWater => (Call::Death, true),
            other => (other, false),
        }
    }
}

pub(super) fn vary(rng: &mut Rng, x: f32, by: f32) -> f32 {
    x * (1.0 + by * rng.signed())
}

/// A wolf howling, far off on a cold night: a long, nearly pure "oooo" that
/// slides up a fifth or so, holds with a slow waver, and falls away, the
/// lips rounded and the jaw opening a little at its height; often a second,
/// shorter one after it.
pub fn howl(seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed ^ 0x40A1);
    let mut voice = Mob::Wolf.voice();
    voice.f0 *= 0.85 + 0.25 * rng.unit();
    let mut segments = Vec::new();
    for n in 0..1 + usize::from(rng.unit() < 0.6) {
        let mut h = Seg::new(vary(&mut rng, if n == 0 { 2.6 } else { 1.6 }, 0.15)).vowels(&[
            (0.0, OO),
            (0.45, UH),
            (1.0, OO),
        ]);
        h.f0 = k(&[
            (0.0, 0.75),
            (0.25, 1.15),
            (0.6, 1.2),
            (0.85, 1.05),
            (1.0, 0.7),
        ]);
        h.lips = k(&[(0.0, 0.3), (0.45, 0.6), (1.0, 0.25)]);
        // A soft, smooth glottal pulse: few overtones, nearly a pure tone.
        h.rd = c(2.2);
        h.breath = c(0.1);
        h.tremolo = (vary(&mut rng, 4.5, 0.2), 0.12);
        h.jitter = 0.006;
        h.loud = k(&[(0.0, 0.0), (0.15, 0.8), (0.5, 1.0), (0.85, 0.8), (1.0, 0.0)]);
        h.gap = vary(&mut rng, 0.6, 0.3);
        segments.push(h);
    }
    finish(perform(&mut rng, voice, &segments), 0.4)
}

/// A call of `mob`, made up from `seed`. Calls a mob doesn't have fall back
/// to its ambient one.
pub fn voice(mob: Mob, call: Call, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let (call, in_water) = call.dry();
    let three = match call {
        Call::Hurt => Call3::Hurt,
        Call::Death => Call3::Death,
        _ => Call3::Ambient,
    };
    let (sound, peak) = match mob {
        Mob::Skeleton => (creatures::skeleton(&mut rng, three, Bones::Plain), 0.5),
        Mob::Stray => (creatures::skeleton(&mut rng, three, Bones::Frosted), 0.5),
        Mob::Bogged => (creatures::skeleton(&mut rng, three, Bones::Mossy), 0.5),
        Mob::Spider => (creatures::spider(&mut rng, three, 1.0), 0.5),
        Mob::CaveSpider => (creatures::spider(&mut rng, three, 1.5), 0.45),
        Mob::Bat => (creatures::bat(&mut rng, three), 0.4),
        Mob::Bee => {
            let sound = match call {
                Call::Hurt => creatures::bee_cry(&mut rng, false),
                Call::Death => creatures::bee_cry(&mut rng, true),
                // A loop: no fades or tail, so it joins up.
                _ => {
                    let mut buzz = creatures::bee_loop(&mut rng, call == Call::LoopAggressive);
                    let loudest = buzz.iter().fold(0f32, |m, x| m.max(x.abs()));
                    buzz.iter_mut().for_each(|x| *x *= 0.35 / loudest.max(1e-6));
                    return buzz;
                }
            };
            (sound, 0.5)
        }
        Mob::Slime | Mob::SlimeSmall => {
            let slime = match call {
                Call::Jump => creatures::Slime::Jump,
                Call::Attack => creatures::Slime::Attack,
                Call::Hurt => creatures::Slime::Hurt,
                Call::Death => creatures::Slime::Death,
                _ => creatures::Slime::Squish,
            };
            let size = if mob == Mob::Slime { 1.0 } else { 0.4 };
            (creatures::slime(&mut rng, slime, size), 0.5)
        }
        Mob::IronGolem => (creatures::iron_golem(&mut rng, three, seed), 0.6),
        Mob::SnowGolem => (creatures::snow_golem(three, seed), 0.45),
        Mob::Silverfish => (creatures::spider(&mut rng, three, 2.5), 0.4),
        Mob::Endermite => (creatures::spider(&mut rng, three, 3.0), 0.4),
        Mob::Frog if three == Call3::Ambient => (crate::frog(5.0 + 3.0 * rng.unit(), seed), 0.5),
        Mob::Guardian | Mob::ElderGuardian if call == Call::Flop => {
            (creatures::flop(&mut rng, mob == Mob::ElderGuardian), 0.55)
        }
        Mob::Phantom if call == Call::Swoop || call == Call::Flap => {
            (creatures::phantom_air(&mut rng, call == Call::Swoop), 0.45)
        }
        Mob::Creeper => {
            let call = match call {
                Call::Hurt => CreeperCall::Hurt,
                Call::Death => CreeperCall::Death,
                _ => CreeperCall::Primed,
            };
            (creatures::creeper(&mut rng, call), 0.55)
        }
        _ => {
            let segments = script(&mut rng, mob, call);
            let mut voice = mob.voice();
            voice.f0 *= 1.0 + 0.06 * rng.signed();
            let mut sound = perform(&mut rng, voice, &segments);
            let submerged =
                matches!(mob, Mob::Guardian | Mob::ElderGuardian) && call != Call::AmbientLand;
            if submerged {
                // Guardians live in water: a few bubbles rise as they call.
                // (Earshot muffles it if the listener is under water too.)
                sound = creatures::bubbles(&mut rng, sound, 3);
            }
            if mob == Mob::Drowned {
                sound = if in_water {
                    creatures::underwater(&mut rng, sound)
                } else {
                    creatures::gurgle(&mut rng, sound)
                };
            }
            let hostile = matches!(
                mob,
                Mob::Zombie
                    | Mob::Husk
                    | Mob::Drowned
                    | Mob::ZombieVillager
                    | Mob::Enderman
                    | Mob::Witch
                    | Mob::Phantom
                    | Mob::Pillager
                    | Mob::Vindicator
                    | Mob::Evoker
                    | Mob::Illusioner
                    | Mob::Ravager
                    | Mob::Vex
                    | Mob::Guardian
                    | Mob::ElderGuardian
            );
            let peak = match call {
                Call::Hurt => 0.7,
                Call::Death => 0.6,
                Call::Breath if hostile => 0.3,
                Call::BreathHeavy if hostile => 0.5,
                Call::AmbientNear if hostile => 0.65,
                Call::Growl if hostile => 0.7,
                _ if hostile => 0.45,
                _ => 0.5,
            };
            (sound, peak)
        }
    };
    finish(sound, peak)
}

/// A sharp, rough cry, then sometimes a gasp.
fn hurt(rng: &mut Rng, mob: Mob) -> Vec<Seg> {
    let mut cry = Seg::new(vary(rng, 0.28, 0.2)).vowels(&[(0.0, AA), (1.0, UH)]);
    cry.f0 = k(&[(0.0, 1.4), (0.25, 1.8), (1.0, 1.25)]);
    cry.rd = c(0.6);
    cry.loud = k(&[(0.0, 0.0), (0.06, 1.0), (0.6, 0.8), (1.0, 0.0)]);
    cry.chaos = k(&[(0.0, 0.5), (0.5, 0.3), (1.0, 0.1)]);
    cry.sub = c(0.2);
    cry.breath = c(0.2);
    match mob {
        Mob::Pig => {
            // A squeal.
            cry.f0 = k(&[(0.0, 5.0), (0.3, 6.5), (1.0, 5.0)]);
            cry.mouth = vec![(0.0, EE), (1.0, EH)];
            cry.secs = vary(rng, 0.4, 0.2);
        }
        Mob::Wolf => {
            // A yelp.
            cry.f0 = k(&[(0.0, 1.5), (0.2, 2.1), (1.0, 1.3)]);
            cry.mouth = vec![(0.0, AA), (1.0, OO)];
            cry.secs = vary(rng, 0.2, 0.2);
        }
        Mob::Villager
        | Mob::WanderingTrader
        | Mob::Pillager
        | Mob::Vindicator
        | Mob::Evoker
        | Mob::Illusioner => cry.mouth = vec![(0.0, OO), (1.0, OH)],
        Mob::Horse | Mob::Mule => {
            cry.bi = c(0.5);
            cry.bi_ratio = 0.31;
        }
        Mob::Enderman => cry.ingressive = true,
        Mob::Cat => cry.mouth = vec![(0.0, EE), (0.4, AA), (1.0, OO)],
        _ => {}
    }
    let mut segments = vec![cry];
    if rng.unit() < 0.5 {
        let mut gasp = Seg::new(vary(rng, 0.15, 0.2)).vowels(&[(0.0, UH)]);
        gasp.ingressive = true;
        gasp.voicing = c(0.3);
        gasp.breath = c(0.6);
        gasp.f0 = c(1.3);
        gasp.loud = k(&[(0.0, 0.0), (0.3, 0.5), (1.0, 0.0)]);
        segments.push(gasp);
    }
    segments
}

/// A falling, fading cry that turns breathy and creaks out, and a last
/// breath.
fn death(rng: &mut Rng) -> Vec<Seg> {
    let mut cry = Seg::new(vary(rng, 0.9, 0.15)).vowels(&[(0.0, AA), (1.0, OO)]);
    cry.f0 = k(&[(0.0, 1.3), (1.0, 0.5)]);
    cry.rd = k(&[(0.0, 1.0), (1.0, 2.4)]);
    cry.loud = k(&[(0.0, 0.0), (0.05, 1.0), (1.0, 0.0)]);
    cry.chaos = c(0.3);
    cry.creak = k(&[(0.0, 0.0), (0.7, 0.0), (1.0, 0.8)]);
    cry.breath = k(&[(0.0, 0.2), (1.0, 0.5)]);
    let mut exhale = Seg::new(vary(rng, 0.4, 0.2)).vowels(&[(0.0, UH)]);
    exhale.voicing = c(0.0);
    exhale.breath = c(0.4);
    exhale.loud = k(&[(0.0, 0.3), (1.0, 0.0)]);
    vec![cry, exhale]
}

/// A zombie's groan: slow, rough and breathy, with fry at its ends, and
/// sometimes a voiced in-breath after it. `dry` rasps it (a husk), `nasal`
/// keeps the villager in it.
fn groan(rng: &mut Rng, dry: bool, nasal: bool) -> Vec<Seg> {
    let mut g =
        Seg::new(vary(rng, 1.3, 0.25)).vowels(&[(0.0, UH), (0.3, AA), (0.7, OH), (1.0, UH)]);
    g.lips = c(0.7);
    g.f0 = k(&[(0.0, 1.05), (0.5, 0.95), (1.0, 0.7)]);
    g.rd = k(&[(0.0, 1.8), (1.0, 2.4)]);
    g.sub = c(0.35);
    g.chaos = k(&[(0.0, 0.1), (0.4, 0.35), (0.8, 0.2)]);
    g.creak = k(&[(0.0, 0.4), (0.15, 0.0), (0.75, 0.0), (1.0, 0.7)]);
    g.breath = c(0.3);
    g.sidebands = c(0.15);
    g.sideband_hz = 45.0 + 35.0 * rng.unit();
    g.jitter = 0.025;
    g.loud = k(&[(0.0, 0.0), (0.2, 0.8), (0.5, 1.0), (0.85, 0.7), (1.0, 0.0)]);
    if dry {
        g.rd = c(2.6);
        g.breath = c(0.55);
        g.fric = c(0.25);
        g.fric_at = (0.25, 0.5);
        g.creak = k(&[(0.0, 0.6), (0.2, 0.1), (0.7, 0.1), (1.0, 0.8)]);
    }
    if nasal {
        g.velum = c(0.8);
        g.lips = k(&[(0.0, 0.0), (0.6, 0.2), (1.0, 0.0)]);
        g.mouth = vec![(0.0, UH), (1.0, OO)];
        g.secs = vary(rng, 0.9, 0.2);
    }
    let mut segments = vec![g];
    if rng.unit() < 0.4 {
        let mut catch = Seg::new(vary(rng, 0.35, 0.2)).vowels(&[(0.0, UH)]);
        catch.ingressive = true;
        catch.voicing = c(0.5);
        catch.breath = c(0.5);
        catch.f0 = c(1.2);
        catch.sub = c(0.3);
        catch.loud = k(&[(0.0, 0.0), (0.3, 0.6), (1.0, 0.0)]);
        segments[0].gap = vary(rng, 0.12, 0.3);
        segments.push(catch);
    }
    segments
}

/// A zombie right on top of you: a wet, rattling gasp in, then a snarl
/// through bared teeth, pressed hard, rough with subharmonics and chaos,
/// that breaks up into fry, and sometimes a choke as it runs out of air.
fn snarl(rng: &mut Rng, dry: bool, nasal: bool) -> Vec<Seg> {
    let mut gasp = Seg::new(vary(rng, 0.3, 0.25)).vowels(&[(0.0, UH), (1.0, AA)]);
    gasp.ingressive = true;
    gasp.voicing = c(0.45);
    gasp.breath = c(0.7);
    gasp.creak = c(0.8);
    gasp.chaos = c(0.5);
    gasp.f0 = c(0.75);
    gasp.loud = k(&[(0.0, 0.0), (0.5, 0.7), (1.0, 0.2)]);
    gasp.gap = vary(rng, 0.04, 0.4);
    let mut snarl =
        Seg::new(vary(rng, 0.9, 0.25)).vowels(&[(0.0, EH), (0.25, AE), (0.8, AA), (1.0, UH)]);
    snarl.lips = c(1.0);
    snarl.rd = c(if dry { 1.0 } else { 0.5 });
    snarl.f0 = k(&[(0.0, 0.8), (0.25, 1.0), (0.7, 0.85), (1.0, 0.6)]);
    snarl.sub = c(0.6);
    snarl.chaos = k(&[(0.0, 0.4), (0.3, 0.7), (1.0, 0.5)]);
    snarl.sidebands = c(0.4);
    snarl.sideband_hz = 30.0 + 25.0 * rng.unit();
    snarl.creak = k(&[(0.0, 0.3), (0.6, 0.2), (1.0, 0.9)]);
    snarl.breath = c(if dry { 0.6 } else { 0.4 });
    snarl.jitter = 0.04;
    snarl.loud = k(&[(0.0, 0.0), (0.1, 0.8), (0.35, 1.0), (0.8, 0.85), (1.0, 0.0)]);
    if dry {
        snarl.fric = c(0.3);
        snarl.fric_at = (0.25, 0.5);
    }
    if nasal {
        snarl.velum = c(0.5);
    }
    let mut segments = vec![gasp, snarl];
    if rng.unit() < 0.5 {
        let mut choke = Seg::new(vary(rng, 0.18, 0.3)).vowels(&[(0.0, UH)]);
        choke.ingressive = true;
        choke.voicing = c(0.6);
        choke.creak = c(1.0);
        choke.chaos = c(0.6);
        choke.f0 = c(1.1);
        choke.breath = c(0.5);
        choke.loud = k(&[(0.0, 0.0), (0.2, 0.7), (1.0, 0.0)]);
        segments[1].gap = vary(rng, 0.08, 0.4);
        segments.push(choke);
    }
    segments
}

/// A zombie breathing: rough, half-voiced breaths out and in. Heavy, they
/// come faster and harder, more voiced, catching in the throat.
fn breathe(rng: &mut Rng, heavy: bool, dry: bool, nasal: bool) -> Vec<Seg> {
    let cycles = if heavy {
        2 + (rng.unit() * 2.0) as usize
    } else {
        1 + (rng.unit() * 2.0) as usize
    };
    let mut segments = Vec::new();
    for _ in 0..cycles {
        let (out_s, in_s) = if heavy { (0.35, 0.3) } else { (0.6, 0.5) };
        let mut out = Seg::new(vary(rng, out_s, 0.2)).vowels(&[(0.0, UH), (1.0, AA)]);
        out.voicing = c(if heavy { 0.35 } else { 0.12 });
        out.breath = c(if heavy { 0.8 } else { 0.55 });
        out.rd = c(2.4);
        out.creak = k(&[(0.0, 0.5), (0.4, 0.2), (1.0, 0.6)]);
        out.sub = c(0.3);
        out.chaos = c(if heavy { 0.4 } else { 0.15 });
        out.lips = c(0.6);
        out.f0 = c(0.9);
        out.loud = k(&[(0.0, 0.0), (0.2, 1.0), (1.0, 0.0)]);
        out.gap = vary(rng, if heavy { 0.05 } else { 0.15 }, 0.3);
        if dry {
            out.fric = c(0.3);
            out.fric_at = (0.25, 0.5);
        }
        if nasal {
            out.velum = c(0.6);
            out.lips = c(0.15);
        }
        let mut inward = out.clone();
        inward.secs = vary(rng, in_s, 0.2);
        inward.ingressive = true;
        inward.voicing = c(if heavy { 0.25 } else { 0.08 });
        inward.fric = c(if dry { 0.4 } else { 0.12 });
        inward.fric_at = (0.25, 0.6);
        inward.loud = k(&[(0.0, 0.0), (0.4, 0.8), (1.0, 0.0)]);
        inward.gap = vary(rng, if heavy { 0.1 } else { 0.35 }, 0.3);
        segments.push(out);
        segments.push(inward);
    }
    segments
}

fn script(rng: &mut Rng, mob: Mob, call: Call) -> Vec<Seg> {
    if let Some(segments) = mobs2::script(rng, mob, call) {
        return segments;
    }
    match (mob, call) {
        (_, Call::Hurt) => hurt(rng, mob),
        (_, Call::Death) => death(rng),
        (
            Mob::Zombie | Mob::Husk | Mob::Drowned | Mob::ZombieVillager,
            Call::Breath | Call::BreathHeavy,
        ) => breathe(
            rng,
            call == Call::BreathHeavy,
            mob == Mob::Husk,
            mob == Mob::ZombieVillager,
        ),
        (Mob::Zombie | Mob::Husk | Mob::Drowned | Mob::ZombieVillager, Call::Growl) => {
            snarl(rng, mob == Mob::Husk, mob == Mob::ZombieVillager)
        }
        (Mob::Zombie | Mob::Husk | Mob::Drowned | Mob::ZombieVillager, _) => {
            let mut g = groan(rng, mob == Mob::Husk, mob == Mob::ZombieVillager);
            if call == Call::AmbientNear {
                // Closer, it strains: pressed harder, rougher, more chaotic,
                // breathing hard after.
                for s in &mut g {
                    s.rd = c(if mob == Mob::Husk { 2.0 } else { 1.2 });
                    s.chaos = k(&[(0.0, 0.3), (0.4, 0.5), (1.0, 0.3)]);
                    s.sub = c(0.45);
                    s.breath = c(0.45);
                    s.f0 = k(&[(0.0, 1.15), (0.5, 1.05), (1.0, 0.8)]);
                }
                g.extend(breathe(
                    rng,
                    true,
                    mob == Mob::Husk,
                    mob == Mob::ZombieVillager,
                ));
            }
            g
        }
        (Mob::Wolf, Call::Growl) => {
            // Rumbling out and in.
            let mut out = Seg::new(vary(rng, 0.8, 0.2)).vowels(&[(0.0, UH)]);
            out.f0 = c(0.22);
            out.rd = c(0.5);
            out.sub = c(0.5);
            out.chaos = c(0.5);
            out.creak = c(0.3);
            out.lips = c(0.5);
            out.sidebands = c(0.2);
            out.sideband_hz = 40.0;
            out.breath = c(0.3);
            let mut inward = out.clone();
            inward.secs = vary(rng, 0.5, 0.2);
            inward.ingressive = true;
            vec![out, inward]
        }
        (Mob::Wolf, Call::Whine) => {
            let mut w = Seg::new(vary(rng, 0.8, 0.25)).vowels(&[(0.0, OO)]);
            w.f0 = k(&[(0.0, 2.0), (0.4, 2.4), (1.0, 1.9)]);
            w.velum = c(1.0);
            w.lips = c(0.2);
            w.tremolo = (6.0, 0.4);
            vec![w]
        }
        (Mob::Wolf, Call::Pant) => pant(rng),
        (Mob::Wolf, _) => {
            if rng.unit() < 0.7 {
                pant(rng)
            } else {
                (0..1 + (rng.unit() * 2.0) as usize)
                    .map(|_| {
                        let mut b = Seg::new(vary(rng, 0.13, 0.2)).vowels(&[(0.0, AA), (1.0, OO)]);
                        b.f0 = k(&[(0.0, 1.1), (1.0, 0.75)]);
                        b.rd = c(0.6);
                        b.chaos = k(&[(0.0, 0.6), (0.4, 0.2)]);
                        b.breath = c(0.4);
                        b.loud = k(&[(0.0, 0.0), (0.08, 1.0), (1.0, 0.0)]);
                        b.gap = vary(rng, 0.25, 0.3);
                        b
                    })
                    .collect()
            }
        }
        (Mob::Cat, Call::Purr) => {
            // The larynx's muscles twitching some 25 times a second, on the
            // out-breath and the in-breath, mouth shut.
            let mut out = Seg::new(vary(rng, 0.9, 0.15)).vowels(&[(0.0, UH)]);
            out.f0 = c(25.0 / 600.0);
            out.rd = c(0.4);
            out.lips = c(0.0);
            out.velum = c(1.0);
            out.breath = c(0.25);
            out.jitter = 0.04;
            let mut inward = out.clone();
            inward.secs = vary(rng, 0.7, 0.15);
            inward.ingressive = true;
            vec![out, inward]
        }
        (Mob::Cat, Call::Hiss) => {
            let mut h = Seg::new(vary(rng, 0.6, 0.2)).vowels(&[(0.0, EE)]);
            h.voicing = c(0.0);
            h.breath = c(0.1);
            h.fric = c(1.0);
            h.fric_at = (0.95, 0.15);
            h.lips = c(0.6);
            h.loud = k(&[(0.0, 0.0), (0.03, 1.0), (0.7, 0.8), (1.0, 0.0)]);
            vec![h]
        }
        (Mob::Cat, Call::Beg) => (0..2)
            .map(|_| {
                let mut m = meow(rng);
                m.secs = vary(rng, 0.35, 0.2);
                m.f0 = k(&[(0.0, 1.05), (0.4, 1.45), (1.0, 1.0)]);
                m.gap = 0.15;
                m
            })
            .collect(),
        (Mob::Cat, _) => vec![meow(rng)],
        (Mob::Cow, _) => {
            // "Mm-ooo": lips shut and the nose open, then a low vowel, with
            // subharmonics at its height, as real moos have.
            let mut m = Seg::new(vary(rng, 1.5, 0.2)).vowels(&[(0.0, OO), (0.4, OH), (0.8, OO)]);
            m.lips = k(&[(0.0, 0.0), (0.2, 0.0), (0.35, 1.0)]);
            m.velum = k(&[(0.0, 1.0), (0.25, 1.0), (0.4, 0.1)]);
            m.f0 = k(&[(0.0, 0.9), (0.3, 1.25), (0.75, 1.1), (1.0, 0.8)]);
            m.rd = c(1.1);
            m.sub = k(&[(0.3, 0.0), (0.5, 0.25), (0.7, 0.0)]);
            m.breath = c(0.08);
            m.loud = k(&[(0.0, 0.0), (0.15, 0.6), (0.4, 1.0), (0.85, 0.8), (1.0, 0.0)]);
            vec![m]
        }
        (Mob::Pig, _) => (0..2 + (rng.unit() * 3.0) as usize)
            .map(|_| {
                let mut g = Seg::new(vary(rng, 0.16, 0.25)).vowels(&[(0.0, OO)]);
                g.lips = c(0.2);
                g.velum = c(0.8);
                g.f0 = k(&[(0.0, 1.1), (1.0, 0.8)]);
                g.rd = c(0.5);
                g.sub = c(0.5);
                g.chaos = c(0.3);
                g.breath = c(0.35);
                g.loud = k(&[(0.0, 0.0), (0.1, 1.0), (1.0, 0.0)]);
                g.gap = vary(rng, 0.09, 0.35);
                g
            })
            .collect(),
        (Mob::Sheep, _) => {
            // "Baa", lips parting, its pitch and loudness trembling.
            let mut b = Seg::new(vary(rng, 0.85, 0.2)).vowels(&[(0.0, EH), (0.2, AE), (1.0, AA)]);
            b.lips = k(&[(0.0, 0.0), (0.06, 1.0)]);
            b.f0 = k(&[(0.0, 0.95), (0.2, 1.1), (1.0, 0.85)]);
            b.tremolo = (vary(rng, 7.5, 0.15), 0.9);
            b.sidebands = c(0.35);
            b.sideband_hz = b.tremolo.0;
            b.rd = c(1.2);
            b.breath = c(0.15);
            b.sub = c(0.1);
            vec![b]
        }
        (Mob::Chicken, _) => {
            let mut clucks: Vec<Seg> = (0..3 + (rng.unit() * 3.0) as usize)
                .map(|_| {
                    let mut s = Seg::new(vary(rng, 0.075, 0.2)).vowels(&[(0.0, OH), (1.0, AA)]);
                    s.f0 = k(&[(0.0, 1.0), (1.0, 0.85)]);
                    s.rd = c(0.7);
                    s.chaos = c(0.25);
                    s.sub = c(0.2);
                    s.breath = c(0.15);
                    s.loud = k(&[(0.0, 0.0), (0.15, 1.0), (1.0, 0.0)]);
                    s.gap = vary(rng, 0.12, 0.3);
                    s
                })
                .collect();
            if rng.unit() < 0.4 {
                let mut bawk = Seg::new(vary(rng, 0.25, 0.2)).vowels(&[(0.0, OH), (0.3, AA)]);
                bawk.f0 = k(&[(0.0, 1.2), (0.4, 1.4), (1.0, 1.05)]);
                bawk.chaos = c(0.4);
                bawk.rd = c(0.7);
                clucks.push(bawk);
            }
            clucks
        }
        (Mob::Villager, _) => villager(rng),
        // A call this mob doesn't have: its ambient one.
        _ => mobs2::script(rng, mob, Call::Ambient).unwrap_or_else(|| vec![Seg::new(0.3)]),
    }
}

/// A villager to itself, through its big nose: one of a "hrm", a "huh?", a
/// musing "mm-hm-hmm" with a tune to it, a sigh, a throat cleared, a sniff.
fn villager(rng: &mut Rng) -> Vec<Seg> {
    let hum = |rng: &mut Rng, secs: f32, f0: K| {
        let mut h = Seg::new(vary(rng, secs, 0.25)).vowels(&[(0.0, UH)]);
        h.velum = c(1.0);
        h.lips = c(0.0);
        h.rd = c(1.2);
        h.f0 = f0;
        h.jitter = 0.015;
        h
    };
    let roll = rng.unit();
    if roll < 0.25 {
        // "Hrm", with fry at its start.
        let f0 = if rng.unit() < 0.5 {
            k(&[(0.0, 1.12), (1.0, 0.84)])
        } else {
            k(&[(0.0, 0.92), (0.5, 1.16), (1.0, 0.88)])
        };
        let mut h = hum(rng, 0.55, f0);
        h.creak = k(&[(0.0, 0.5), (0.3, 0.0)]);
        vec![h]
    } else if roll < 0.4 {
        // "Huh?": open and breathy, rising.
        let mut h = hum(rng, 0.3, k(&[(0.0, 0.95), (1.0, 1.25)]));
        h.velum = c(0.0);
        h.lips = c(1.0);
        h.voicing = c(0.6);
        h.breath = c(0.4);
        vec![h]
    } else if roll < 0.7 {
        // Musing: two or three hums, a little tune, the last one trailing off.
        let n = 2 + (rng.unit() * 2.0) as usize;
        let mut level = 1.0 + 0.1 * rng.signed();
        (0..n)
            .map(|i| {
                let last = i + 1 == n;
                let step = 0.12 * rng.signed();
                let f0 = if last {
                    k(&[(0.0, level), (1.0, level * 0.82)])
                } else {
                    k(&[(0.0, level), (1.0, level + step)])
                };
                level += step;
                let mut h = hum(rng, if last { 0.4 } else { 0.18 }, f0);
                if last {
                    h.creak = k(&[(0.0, 0.0), (0.6, 0.0), (1.0, 0.5)]);
                }
                h.gap = vary(rng, 0.07, 0.4);
                h
            })
            .collect()
    } else if roll < 0.82 {
        // A sigh: breath in through the nose, out through the mouth, falling.
        let mut inward = hum(rng, 0.35, c(1.0));
        inward.ingressive = true;
        inward.voicing = c(0.0);
        inward.breath = c(0.8);
        inward.gap = 0.05;
        let mut out = hum(rng, 0.7, k(&[(0.0, 1.1), (1.0, 0.75)]));
        out.velum = c(0.3);
        out.lips = c(0.8);
        out.mouth = vec![(0.0, AA), (1.0, UH)];
        out.voicing = k(&[(0.0, 0.6), (1.0, 0.1)]);
        out.breath = c(0.6);
        out.creak = k(&[(0.0, 0.0), (0.7, 0.0), (1.0, 0.6)]);
        vec![inward, out]
    } else if roll < 0.92 {
        // Clearing its throat: a rasp at the back of the mouth, then "hm".
        let mut rasp = Seg::new(vary(rng, 0.14, 0.2))
            .vowels(&[(0.0, UH)])
            .voicing(0.15)
            .fric(0.7, 0.35, 0.3)
            .loud(&[(0.0, 0.0), (0.2, 1.0), (1.0, 0.0)])
            .gap(0.04);
        rasp.creak = c(0.6);
        let second = rasp.clone().gap(0.1);
        let mut hm = hum(rng, 0.25, k(&[(0.0, 1.0), (1.0, 0.9)]));
        hm.creak = k(&[(0.0, 0.4), (0.4, 0.0)]);
        if rng.unit() < 0.5 {
            vec![rasp, second, hm]
        } else {
            vec![rasp, hm]
        }
    } else {
        // A sniff or two, then a satisfied "hm".
        let mut s = speech::sniff(2 + (rng.unit() * 2.0) as usize, 7.0, 0.0, rng);
        s.push(hum(rng, 0.25, k(&[(0.0, 0.95), (1.0, 1.1)])));
        s
    }
}

fn meow(rng: &mut Rng) -> Seg {
    // "Mi-a-ow": lips part from an "m", the tongue slides back, the lips round.
    let mut m = Seg::new(vary(rng, 0.7, 0.25)).vowels(&[(0.0, EE), (0.3, AA), (0.85, OO)]);
    m.lips = k(&[(0.0, 0.0), (0.12, 1.0)]);
    m.velum = k(&[(0.0, 1.0), (0.15, 0.0)]);
    m.f0 = k(&[(0.0, 0.92), (0.35, 1.33), (1.0, 0.83)]);
    m.rd = c(1.0);
    m.breath = c(0.08);
    m
}

/// Panting: short breaths out and in, about three a second.
fn pant(rng: &mut Rng) -> Vec<Seg> {
    (0..4 + (rng.unit() * 4.0) as usize)
        .map(|i| {
            let mut b =
                Seg::new(vary(rng, 0.11, 0.2)).vowels(&[(0.0, if i % 2 == 0 { AA } else { EH })]);
            b.voicing = c(0.05);
            b.breath = c(0.8);
            b.ingressive = i % 2 == 1;
            b.loud = k(&[(0.0, 0.0), (0.3, 0.6), (1.0, 0.0)]);
            b.gap = vary(rng, 0.08, 0.2);
            b
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;

    fn crossings(s: &[f32]) -> f32 {
        s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f32 / s.len() as f32
    }

    fn rms(s: &[f32]) -> f32 {
        (s.iter().map(|x| x * x).sum::<f32>() / s.len().max(1) as f32).sqrt()
    }

    #[test]
    fn every_mob_has_every_call() {
        for mob in Mob::ALL {
            for call in Call::ALL {
                let s = voice(mob, call, 3);
                assert!(s.iter().all(|x| x.is_finite()), "{mob:?} {call:?}");
                assert!(
                    s.len() > SAMPLE_RATE as usize / 20 && s.len() < 4 * SAMPLE_RATE as usize,
                    "{mob:?} {call:?}: {} samples",
                    s.len()
                );
                let peak = s.iter().fold(0f32, |p, x| p.max(x.abs()));
                assert!(peak > 0.25 && peak <= 0.71, "{mob:?} {call:?}: {peak}");
            }
        }
    }

    #[test]
    fn a_cow_is_deeper_than_a_cat_and_a_zombie_groans_long() {
        let cow = voice(Mob::Cow, Call::Ambient, 1);
        let cat = voice(Mob::Cat, Call::Ambient, 1);
        assert!(crossings(&cow) < crossings(&cat));
        assert!(voice(Mob::Zombie, Call::Ambient, 2).len() > SAMPLE_RATE as usize * 8 / 10);
        // Heavy breathing is louder than steady breathing.
        let light = voice(Mob::Zombie, Call::Breath, 3);
        let heavy = voice(Mob::Zombie, Call::BreathHeavy, 3);
        assert!(rms(&heavy) > rms(&light));
    }

    #[test]
    fn a_bee_loops_seamlessly_and_a_witch_cackles_in_bursts() {
        let buzz = voice(Mob::Bee, Call::Loop, 1);
        assert_eq!(buzz.len(), SAMPLE_RATE as usize);
        let step = (buzz[0] - buzz[buzz.len() - 1]).abs();
        let typical = buzz
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0f32, f32::max);
        assert!(
            step <= typical,
            "loop seam {step} against steps up to {typical}"
        );
        // A cackle: separate bursts, so its envelope dips again and again.
        let cackle = voice(Mob::Witch, Call::Celebrate, 2);
        let window = SAMPLE_RATE as usize / 100;
        let envelope: Vec<f32> = cackle.chunks(window).map(rms).collect();
        let top = envelope.iter().fold(0f32, |m, x| m.max(*x));
        let dips = envelope
            .windows(2)
            .filter(|w| w[0] > 0.3 * top && w[1] <= 0.3 * top)
            .count();
        assert!(dips >= 4, "{dips} bursts");
    }

    #[test]
    fn a_donkey_brays_in_and_out() {
        let bray = voice(Mob::Donkey, Call::Ambient, 1);
        assert!(bray.len() > SAMPLE_RATE as usize, "{} samples", bray.len());
        let horse = voice(Mob::Horse, Call::Ambient, 1);
        assert!(horse.len() > SAMPLE_RATE as usize / 2);
    }

    #[test]
    fn a_creepers_fuse_swells_and_bones_click() {
        let fuse = voice(Mob::Creeper, Call::Primed, 1);
        let half = fuse.len() / 2;
        assert!(rms(&fuse[half..]) > 1.5 * rms(&fuse[..half]));
        let bones = voice(Mob::Skeleton, Call::Ambient, 1);
        assert!(bones.iter().fold(0f32, |p, x| p.max(x.abs())) > 5.0 * rms(&bones));
    }
}
