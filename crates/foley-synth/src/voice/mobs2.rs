//! Calls for the everyday animals (M8) and the rarer Overworld hostiles
//! (M9).

use super::score::{AA, AE, EE, EH, OH, OO, SCHWA, Seg, UH};
use super::speech::{Babble, babble, huff, laugh, sniff};
use super::{Call, Mob, vary};
use crate::Rng;

/// The segments for `mob`'s `call`, or None for the shared hurt and death.
pub(super) fn script(rng: &mut Rng, mob: Mob, call: Call) -> Option<Vec<Seg>> {
    let segments = match (mob, call) {
        // Horses: a whinny is two voices at once (Briefer et al. 2015): a
        // high one sliding down and a low one under it, tremulous.
        (Mob::Horse | Mob::Mule, Call::Ambient | Call::Angry) => {
            let angry = call == Call::Angry;
            let mut w = Seg::new(vary(rng, 1.2, 0.2))
                .vowels(&[(0.0, EE), (0.3, EH), (1.0, AA)])
                .f0(&[(0.0, 1.3), (0.15, 1.5), (0.6, 1.1), (1.0, 0.7)])
                .rd(0.8)
                .bi(0.6, 0.31)
                .tremolo(9.0, 0.8)
                .chaos(if angry { 0.4 } else { 0.15 })
                .breath(0.2)
                .loud(&[(0.0, 0.0), (0.05, 1.0), (0.7, 0.8), (1.0, 0.0)]);
            if mob == Mob::Mule {
                w.secs *= 0.8;
            }
            let mut v = vec![w];
            if angry || rng.unit() < 0.4 {
                v.extend(sniff(2, 4.0, 0.6, rng));
            }
            v
        }
        (Mob::Horse | Mob::Mule | Mob::Donkey, Call::Snort) => {
            sniff(1 + (rng.unit() * 2.0) as usize, 3.0, 0.7, rng)
        }
        // A donkey's bray: voicing on the in-breath ("hee", high) and the
        // out-breath ("haw", low and rough), over and over.
        (Mob::Donkey, Call::Ambient | Call::Angry) => {
            let mut v = Vec::new();
            for _ in 0..3 + (rng.unit() * 3.0) as usize {
                v.push(
                    Seg::new(vary(rng, 0.35, 0.2))
                        .vowels(&[(0.0, EE)])
                        .inward()
                        .f0(&[(0.0, 2.3), (1.0, 2.6)])
                        .rd(1.0)
                        .chaos(0.3)
                        .breath(0.3),
                );
                v.push(
                    Seg::new(vary(rng, 0.45, 0.2))
                        .vowels(&[(0.0, AA), (1.0, OH)])
                        .f0(&[(0.0, 1.0), (1.0, 0.8)])
                        .rd(0.6)
                        .sub(0.4)
                        .chaos(0.4)
                        .breath(0.25),
                );
            }
            v
        }
        // Goats: a nasal, trembling bleat; screaming goats, a loud, rough
        // one.
        (Mob::Goat, Call::Ambient | Call::Scream) => {
            let scream = call == Call::Scream;
            vec![
                Seg::new(vary(rng, if scream { 0.9 } else { 0.7 }, 0.2))
                    .vowels(&[(0.0, EH), (0.2, AE), (1.0, AA)])
                    .lips(&[(0.0, 0.0), (0.06, 1.0)])
                    .velum(&[(0.0, 0.5)])
                    .f0(if scream {
                        &[(0.0, 1.6), (0.3, 2.0), (1.0, 1.4)]
                    } else {
                        &[(0.0, 1.0), (0.3, 1.1), (1.0, 0.85)]
                    })
                    .tremolo(if scream { 6.0 } else { 9.0 }, 1.0)
                    .chaos(if scream { 0.6 } else { 0.15 })
                    .sub(if scream { 0.3 } else { 0.1 })
                    .rd(if scream { 0.6 } else { 1.1 }),
            ]
        }
        // Llamas hum, mouth shut; angry, an alarm call, high and staccato.
        (Mob::Llama, Call::Ambient) => vec![
            Seg::new(vary(rng, 0.8, 0.25))
                .vowels(&[(0.0, UH)])
                .nasal()
                .f0(&[(0.0, 0.9), (0.5, 1.2), (1.0, 0.85)])
                .rd(1.3)
                .breath(0.1),
        ],
        (Mob::Llama, Call::Angry) => (0..3 + (rng.unit() * 3.0) as usize)
            .map(|_| {
                Seg::new(vary(rng, 0.12, 0.2))
                    .vowels(&[(0.0, EE)])
                    .f0(&[(0.0, 4.0), (1.0, 3.6)])
                    .rd(0.7)
                    .chaos(0.3)
                    .gap(0.08)
            })
            .collect(),
        // Rabbits and armadillos snuffle; a fox chatters (a "gekker"), barks
        // or, rarely, screams as a vixen does.
        (Mob::Rabbit | Mob::Armadillo, Call::Ambient) => {
            sniff(3 + (rng.unit() * 4.0) as usize, 6.0, 0.0, rng)
        }
        (Mob::Fox, Call::Ambient | Call::Aggressive) => {
            if call == Call::Aggressive || rng.unit() < 0.5 {
                // Gekkering: rough, rapid clicks of voice.
                (0..6 + (rng.unit() * 6.0) as usize)
                    .map(|_| {
                        Seg::new(vary(rng, 0.04, 0.2))
                            .vowels(&[(0.0, AA)])
                            .f0(&[(0.0, 0.6)])
                            .rd(0.5)
                            .chaos(0.6)
                            .sub(0.5)
                            .gap(0.03)
                    })
                    .collect()
            } else {
                (0..1 + (rng.unit() * 2.0) as usize)
                    .map(|_| {
                        Seg::new(vary(rng, 0.15, 0.2))
                            .vowels(&[(0.0, OO), (0.3, AA), (1.0, OO)])
                            .f0(&[(0.0, 1.1), (1.0, 0.8)])
                            .rd(0.7)
                            .chaos(0.3)
                            .loud(&[(0.0, 0.0), (0.1, 1.0), (1.0, 0.0)])
                            .gap(0.4)
                    })
                    .collect()
            }
        }
        (Mob::Fox, Call::Screech) => vec![
            Seg::new(vary(rng, 0.8, 0.2))
                .vowels(&[(0.0, EE), (0.3, AA), (1.0, OO)])
                .f0(&[(0.0, 1.4), (0.3, 1.7), (1.0, 1.0)])
                .rd(0.6)
                .chaos(0.5)
                .sub(0.3)
                .breath(0.3),
        ],
        (Mob::Fox, Call::Snort) => sniff(4, 7.0, 0.0, rng),
        (Mob::Fox, Call::Sleep) => vec![
            huff(1.2, SCHWA, 0.3).inward(),
            Seg::new(1.0)
                .vowels(&[(0.0, UH)])
                .nasal()
                .voicing(0.15)
                .breath(0.5)
                .creak(&[(0.0, 0.6)])
                .loud(&[(0.0, 0.0), (0.3, 0.4), (1.0, 0.0)]),
        ],
        // A parrot squawks: harsh and high, its tongue shaping it.
        (Mob::Parrot, Call::Ambient) => {
            let mut v = Vec::new();
            for _ in 0..1 + (rng.unit() * 3.0) as usize {
                v.push(
                    Seg::new(vary(rng, 0.18, 0.3))
                        .vowels(&[(0.0, AE), (1.0, EE)])
                        .f0(&[(0.0, 1.0), (0.5, 1.2), (1.0, 0.9)])
                        .rd(0.5)
                        .chaos(0.5)
                        .sub(0.3)
                        .gap(0.12),
                );
            }
            v
        }
        // Polar bears huff; a cub bleats; warning, a jaw-popping growl.
        (Mob::PolarBear, Call::Ambient) => {
            let mut v = sniff(2, 3.0, 0.0, rng);
            v.push(
                Seg::new(vary(rng, 0.5, 0.3))
                    .vowels(&[(0.0, UH)])
                    .rd(1.6)
                    .sub(0.3)
                    .breath(0.4)
                    .lips(&[(0.0, 0.5)]),
            );
            v
        }
        (Mob::PolarBear, Call::AmbientBaby) => vec![
            Seg::new(vary(rng, 0.5, 0.2))
                .vowels(&[(0.0, EH), (1.0, AA)])
                .f0(&[(0.0, 4.0), (1.0, 3.3)])
                .rd(1.0)
                .chaos(0.2),
        ],
        (Mob::PolarBear | Mob::Ravager, Call::Warning | Call::Roar | Call::Stunned) => {
            let stunned = call == Call::Stunned;
            vec![
                Seg::new(vary(rng, if stunned { 1.2 } else { 1.0 }, 0.2))
                    .vowels(&[(0.0, UH), (0.3, AA), (1.0, OH)])
                    .f0(if stunned {
                        &[(0.0, 0.9), (1.0, 0.6)]
                    } else {
                        &[(0.0, 1.0), (0.3, 1.3), (1.0, 0.9)]
                    })
                    .rd(if stunned { 2.0 } else { 0.5 })
                    .sub(0.5)
                    .chaos(if stunned { 0.3 } else { 0.6 })
                    .creak(&[(0.0, 0.3), (0.2, 0.0), (0.8, 0.0), (1.0, 0.5)])
                    .sidebands(0.2, 40.0)
                    .breath(0.35),
            ]
        }
        (Mob::Ravager, Call::Ambient) => {
            let mut v = sniff(2, 2.5, 0.5, rng);
            v.push(
                Seg::new(vary(rng, 0.6, 0.3))
                    .vowels(&[(0.0, UH)])
                    .rd(1.4)
                    .sub(0.4)
                    .chaos(0.3)
                    .breath(0.4)
                    .lips(&[(0.0, 0.4)]),
            );
            v
        }
        // Pandas bleat and honk; worried, they whimper; aggressive, bark.
        (Mob::Panda, Call::Ambient) => vec![
            Seg::new(vary(rng, 0.5, 0.25))
                .vowels(&[(0.0, EH), (1.0, AA)])
                .lips(&[(0.0, 0.0), (0.08, 1.0)])
                .f0(&[(0.0, 1.0), (1.0, 0.85)])
                .tremolo(7.0, 0.6)
                .rd(1.2),
        ],
        (Mob::Panda, Call::Worried) => vec![
            Seg::new(vary(rng, 0.7, 0.2))
                .vowels(&[(0.0, OO)])
                .velum(&[(0.0, 1.0)])
                .lips(&[(0.0, 0.2)])
                .f0(&[(0.0, 2.0), (0.4, 2.3), (1.0, 1.8)])
                .tremolo(5.0, 0.5),
        ],
        (Mob::Panda, Call::Aggressive) => (0..2)
            .map(|_| {
                Seg::new(vary(rng, 0.15, 0.2))
                    .vowels(&[(0.0, AA), (1.0, OO)])
                    .f0(&[(0.0, 0.7), (1.0, 0.5)])
                    .rd(0.5)
                    .chaos(0.5)
                    .sub(0.4)
                    .gap(0.2)
            })
            .collect(),
        (Mob::Panda, Call::Sneeze) => vec![
            huff(0.4, EE, 0.4).inward(),
            Seg::new(0.12)
                .vowels(&[(0.0, EE)])
                .voicing(0.3)
                .breath(0.9)
                .fric(0.8, 0.9, 0.2)
                .loud(&[(0.0, 1.0), (1.0, 0.0)]),
        ],
        // A camel's groan rumbles and gurgles (through the pouch of its soft
        // palate).
        (Mob::Camel, Call::Ambient) => vec![
            Seg::new(vary(rng, 1.3, 0.25))
                .vowels(&[(0.0, UH), (0.5, OH), (1.0, UH)])
                .velum(&[(0.0, 0.5)])
                .f0(&[(0.0, 1.0), (0.5, 1.2), (1.0, 0.8)])
                .rd(1.4)
                .sub(0.4)
                .sidebands(0.5, 22.0)
                .chaos(0.2)
                .breath(0.3),
        ],
        // Turtles hiss.
        (Mob::Turtle, Call::Ambient | Call::AmbientLand) => {
            vec![huff(vary(rng, 0.6, 0.2), EE, 0.6).fric(0.4, 0.9, 0.3)]
        }
        // An ocelot: a cat's voice, smaller; a chirp more than a meow.
        (Mob::Ocelot, Call::Ambient) => vec![
            Seg::new(vary(rng, 0.25, 0.2))
                .vowels(&[(0.0, EE), (1.0, AA)])
                .lips(&[(0.0, 0.0), (0.1, 1.0)])
                .f0(&[(0.0, 1.0), (0.5, 1.25), (1.0, 1.1)])
                .rd(1.0),
        ],
        // Villagers and wandering traders: "mm-hm!", "mm-mm", "hmm?" and a
        // cheerful "hah!"; the trader older, breathier.
        (Mob::Villager | Mob::WanderingTrader, Call::Yes) => (0..2)
            .map(|i| {
                Seg::new(0.16)
                    .vowels(&[(0.0, UH)])
                    .nasal()
                    .f0(if i == 0 {
                        &[(0.0, 0.95)]
                    } else {
                        &[(0.0, 1.1), (1.0, 1.35)]
                    })
                    .rd(1.2)
                    .gap(0.06)
            })
            .collect(),
        (Mob::Villager | Mob::WanderingTrader, Call::No) => (0..2)
            .map(|i| {
                Seg::new(0.18)
                    .vowels(&[(0.0, UH)])
                    .nasal()
                    .f0(if i == 0 {
                        &[(0.0, 1.15)]
                    } else {
                        &[(0.0, 0.95), (1.0, 0.8)]
                    })
                    .rd(1.0)
                    .creak(&[
                        (0.0, 0.0),
                        (0.6, 0.0),
                        (1.0, if i == 1 { 0.6 } else { 0.0 }),
                    ])
                    .gap(0.07)
            })
            .collect(),
        (Mob::Villager | Mob::WanderingTrader, Call::Trade) => vec![
            Seg::new(vary(rng, 0.7, 0.2))
                .vowels(&[(0.0, UH)])
                .nasal()
                .f0(&[(0.0, 0.9), (0.5, 1.15), (1.0, 1.0)])
                .rd(1.3),
        ],
        (Mob::Villager | Mob::WanderingTrader, Call::Celebrate) => {
            laugh(3, 1.4, 1.1, 0.9, AA, false, rng)
        }
        (Mob::WanderingTrader, Call::Ambient) => vec![
            Seg::new(vary(rng, 0.6, 0.3))
                .vowels(&[(0.0, UH)])
                .nasal()
                .f0(&[(0.0, 1.05), (1.0, 0.85)])
                .rd(1.8)
                .breath(0.3)
                .creak(&[(0.0, 0.3), (0.3, 0.0)]),
        ],
        // Endermen mutter something like speech, but on the in-breath, low,
        // through a tract a little too long for a human voice.
        (Mob::Enderman, Call::Ambient) => babble(
            Babble {
                syllables: 3 + (rng.unit() * 5.0) as usize,
                rate: 3.5,
                from: 1.1,
                to: 0.85,
                rd: 1.6,
                breath: 0.3,
                nasal: 0.1,
                chaos: 0.15,
                ingressive: true,
                creak_end: true,
                lips: 0.7,
            },
            rng,
        ),
        (Mob::Enderman, Call::Stare) => vec![
            Seg::new(1.2)
                .vowels(&[(0.0, UH), (1.0, AA)])
                .inward()
                .f0(&[(0.0, 0.8), (1.0, 1.4)])
                .rd(1.8)
                .sub(0.4)
                .breath(0.5)
                .loud(&[(0.0, 0.0), (0.7, 0.8), (1.0, 0.0)]),
        ],
        (Mob::Enderman, Call::Scream) => vec![
            Seg::new(vary(rng, 1.0, 0.2))
                .vowels(&[(0.0, OO), (0.4, AA), (1.0, UH)])
                .inward()
                .f0(&[(0.0, 2.0), (0.3, 3.0), (1.0, 1.6)])
                .rd(0.9)
                .chaos(0.6)
                .sub(0.4)
                .breath(0.4),
        ],
        // A witch cackles: "hee-hee-hee", falling, pressed and creaky, and a
        // gasp.
        (Mob::Witch, Call::Ambient) => {
            if rng.unit() < 0.6 {
                laugh(
                    4 + (rng.unit() * 4.0) as usize,
                    1.2,
                    0.85,
                    0.7,
                    EE,
                    true,
                    rng,
                )
            } else {
                babble(
                    Babble {
                        syllables: 2 + (rng.unit() * 3.0) as usize,
                        rate: 5.0,
                        from: 1.1,
                        to: 0.9,
                        rd: 1.2,
                        breath: 0.25,
                        nasal: 0.3,
                        chaos: 0.1,
                        ingressive: false,
                        creak_end: true,
                        lips: 0.6,
                    },
                    rng,
                )
            }
        }
        (Mob::Witch, Call::Celebrate) => laugh(8, 1.4, 0.9, 0.6, EE, true, rng),
        // A phantom screeches: high, harsh, chaotic.
        (Mob::Phantom, Call::Ambient) => vec![
            Seg::new(vary(rng, 0.5, 0.25))
                .vowels(&[(0.0, AE), (1.0, EH)])
                .f0(&[(0.0, 1.0), (0.3, 1.15), (1.0, 0.8)])
                .rd(0.6)
                .chaos(0.55)
                .sub(0.3)
                .breath(0.2),
        ],
        // Illagers grumble in something like speech, through the nose,
        // under their breath; they cheer at a raid.
        (Mob::Pillager | Mob::Vindicator | Mob::Evoker | Mob::Illusioner, Call::Ambient) => babble(
            Babble {
                syllables: 2 + (rng.unit() * 4.0) as usize,
                rate: 4.5,
                from: 1.05,
                to: 0.85,
                rd: 1.3,
                breath: 0.15,
                nasal: 0.5,
                chaos: 0.05,
                ingressive: false,
                creak_end: rng.unit() < 0.5,
                lips: 0.45,
            },
            rng,
        ),
        (
            Mob::Pillager | Mob::Vindicator | Mob::Evoker | Mob::Illusioner | Mob::Ravager,
            Call::Celebrate,
        ) => {
            if mob == Mob::Ravager {
                return script(rng, mob, Call::Roar);
            }
            vec![
                Seg::new(vary(rng, 0.45, 0.2))
                    .vowels(&[(0.0, AA), (1.0, OH)])
                    .f0(&[(0.0, 1.2), (0.3, 1.5), (1.0, 1.1)])
                    .rd(0.7)
                    .chaos(0.15),
            ]
        }
        // A vex twitters, small and quick; charging, it rises.
        (Mob::Vex, Call::Ambient | Call::Charge) => {
            let charge = call == Call::Charge;
            (0..if charge {
                1
            } else {
                3 + (rng.unit() * 4.0) as usize
            })
                .map(|_| {
                    Seg::new(vary(rng, if charge { 0.4 } else { 0.07 }, 0.3))
                        .vowels(&[(0.0, EE), (1.0, AE)])
                        .f0(if charge {
                            &[(0.0, 1.0), (1.0, 1.6)]
                        } else {
                            &[(0.0, 1.0), (0.5, 1.3), (1.0, 0.9)]
                        })
                        .rd(0.8)
                        .chaos(if charge { 0.4 } else { 0.1 })
                        .gap(0.05)
                })
                .collect()
        }
        // Guardians moan like whales, gliding; on land they gasp.
        (Mob::Guardian | Mob::ElderGuardian, Call::Ambient) => vec![
            Seg::new(vary(
                rng,
                if mob == Mob::ElderGuardian { 2.2 } else { 1.4 },
                0.2,
            ))
            .vowels(&[(0.0, OO), (0.5, OH), (1.0, UH)])
            .f0(&[(0.0, 1.0), (0.3, 1.6), (0.7, 1.3), (1.0, 0.8)])
            .rd(1.0)
            .tremolo(3.0, 0.4)
            .breath(0.05),
        ],
        (Mob::Guardian | Mob::ElderGuardian, Call::AmbientLand) => (0..2)
            .map(|_| huff(0.3, AA, 0.7).inward().voicing(0.3).gap(0.2))
            .collect(),
        _ => return None,
    };
    Some(segments)
}
