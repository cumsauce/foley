//! The scripted scenes: what each bed is told as the world goes by, as the
//! game's controllers would tell it.

use foley_ffi::{
    FOLEY_BED_FIRE, FOLEY_BED_LAVA, FOLEY_BED_RAIN, FOLEY_BED_THUNDER, FOLEY_BED_WATER,
    FOLEY_BED_WIND, FOLEY_SURFACES,
};

/// A bed through a scene: its kind, how often (ticks) the game sets its
/// scene, its params at a time (seconds), and when lightning strikes it and
/// where, in metres from the listener (y up).
pub struct Track {
    pub name: &'static str,
    pub kind: i32,
    pub every: u32,
    pub params: fn(f32) -> Vec<f32>,
    /// Each flash's (time, x, y, z).
    pub strikes: &'static [(f32, f32, f32, f32)],
    /// What this bed must do beyond not clipping. `None` for the beds whose
    /// sound is made of drops and pops by nature (rain, fire, lava, thunder):
    /// their scenes report loudness and steps for listening and for a
    /// `--baseline`, and fail only on clipping — unless the scenario is about
    /// a fault, when it names one of the checks below.
    pub expect: Option<Expect>,
}

/// The faults a scene is looking for, for a bed that isn't smooth: a jump in
/// the level as the game reads (a scene set with no glide, a bed started
/// loud), a dropout, or a click. Wind and water are always checked for all of
/// these (see `analysis::problems`); a rain, fire, lava or thunder scene names
/// only what it means to prove, so its natural drops and pops don't mask it.
#[derive(Clone, Copy)]
pub struct Expect {
    /// Fail on any click (a sample stepping far above the 10 ms noise).
    pub clicks: bool,
    /// Fail on any dropout (a frame 18 dB under the half second round it).
    pub dropouts: bool,
    /// Fail past this many times the usual level jump at the reads, or `None`.
    pub read_steps: Option<f32>,
}

pub struct Scene {
    pub name: &'static str,
    pub about: &'static str,
    pub seconds: f32,
    pub tracks: Vec<Track>,
}

/// `a` at `t` = 0 to `b` at `t` = 1, eased.
fn ease(a: f32, b: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    a + (b - a) * (3.0 * t * t - 2.0 * t * t * t)
}

/// From `from` to `to` seconds, 0 to 1.
fn during(t: f32, from: f32, to: f32) -> f32 {
    ((t - from) / (to - from)).clamp(0.0, 1.0)
}

/// Rain on a lakeside: half on water, the rest on grass, under `roof` if any.
fn rain_on_water(rate: f32, roof: Option<(f32, f32)>) -> Vec<f32> {
    let mut p = rain(rate, 0.5, 0.0, roof);
    p[1] = 0.5;
    p[1 + 1] = 0.0;
    p
}

/// Rain all of one surface (its [`Surface`] index, as [`rain`]'s roof takes
/// it), under `roof` if any: the ground under a glasshouse, a copper roof, a
/// lake, or grass.
fn rain_on(rate: f32, surface: usize, roof: Option<(f32, f32)>) -> Vec<f32> {
    let mut p = rain(rate, 0.0, 0.0, roof);
    for share in &mut p[1..=FOLEY_SURFACES] {
        *share = 0.0;
    }
    p[1 + surface] = 1.0;
    p
}

/// Rain's params: `rate` mm/h falling on `soft` ground (grass, soil) and
/// `leaves`, the rest on stone, under a wooden `roof` this high if any.
fn rain(rate: f32, soft: f32, leaves: f32, roof: Option<(f32, f32)>) -> Vec<f32> {
    // Surfaces in Foley's order: water, stone, leaves, wood, soft, glass, metal.
    let mut p = vec![0.0; 3 + FOLEY_SURFACES];
    p[0] = rate;
    p[1 + 1] = (1.0 - soft - leaves).max(0.0);
    p[1 + 2] = leaves;
    p[1 + 4] = soft;
    let (roof, height) = roof.unwrap_or((-1.0, 0.0));
    p[1 + FOLEY_SURFACES] = roof;
    p[2 + FOLEY_SURFACES] = height;
    p
}

/// The shared expectation of a bed that must not step or fall silent where it
/// shouldn't, whatever it's made of: no dropout, and no level jump at the
/// game's reads beyond what a scene easing between two values makes. Used for
/// cold starts and for roofs and doorways — places a bed was previously
/// started or switched with no glide, which clicked. (`clicks` is left out:
/// a drop or a pop landing near the ear is a click by this measure, and rain,
/// fire and lava are only drops and pops.)
const NO_STEP: Expect = Expect {
    clicks: false,
    dropouts: true,
    read_steps: Some(1.5),
};

/// Thunder, fire and lava's own: their events (roll segments, crackles,
/// bursts) are sparse and loud by nature, so `dropouts` fires in the gaps
/// between them, and the click detector fires on numerical dust there — a
/// segment tail under the 16-bit floor steps some 12× the gap's own silence,
/// yet is a tenth or less of the roll around it, far under hearing, and
/// whether one lands depends on the strike's seed. What must hold is that the
/// level doesn't step at the game's reads — a scene set with no glide, which
/// is what a shelter changing mid-roll or a fire spreading would do.
const NO_JUMP: Expect = Expect {
    clicks: false,
    dropouts: false,
    read_steps: Some(1.5),
};

pub fn all() -> Vec<Scene> {
    vec![
        Scene {
            name: "forest-walk",
            about: "Open plains into a wood and through it, in a breeze that freshens",
            seconds: 60.0,
            tracks: vec![Track {
                name: "wind",
                kind: FOLEY_BED_WIND,
                every: 5,
                params: |t| {
                    let woods = during(t, 10.0, 20.0) - during(t, 45.0, 55.0);
                    vec![
                        ease(1.5, 6.0, during(t, 25.0, 40.0)),
                        ease(1.0, 0.5, woods),
                        ease(0.0, 0.9, woods),
                    ]
                },
                strikes: &[],
                expect: None,
            }],
        },
        Scene {
            name: "storm",
            about: "Rain comes on over half a minute, a gale with it, lightning, then indoors under a roof",
            seconds: 70.0,
            tracks: vec![
                Track {
                    name: "rain",
                    kind: FOLEY_BED_RAIN,
                    every: 10,
                    params: |t| {
                        let rate = ease(0.0, 60.0, during(t, 2.0, 32.0));
                        let roof = (t > 50.0).then_some((3.0, 4.0));
                        rain_on_water(rate, roof)
                    },
                    strikes: &[],
                    expect: None,
                },
                Track {
                    name: "wind",
                    kind: FOLEY_BED_WIND,
                    every: 5,
                    params: |t| {
                        let inside = during(t, 50.0, 52.0);
                        vec![
                            ease(3.0, 15.0, during(t, 10.0, 35.0)),
                            ease(1.0, 0.05, inside),
                            0.6,
                        ]
                    },
                    strikes: &[],
                    expect: None,
                },
                Track {
                    name: "thunder",
                    kind: FOLEY_BED_THUNDER,
                    every: 10,
                    params: |t| vec![18.0, 85.0, if t > 50.0 { 0.1 } else { 1.0 }],
                    strikes: &[
                        (25.0, 600.0, 0.0, 300.0),
                        (38.0, -900.0, 0.0, 100.0),
                        (55.0, 200.0, 0.0, -800.0),
                    ],
                    expect: None,
                },
            ],
        },
        Scene {
            name: "waterfall-pass",
            about: "Walking past a waterfall and the stream below it, then on",
            seconds: 45.0,
            tracks: vec![Track {
                name: "water",
                kind: FOLEY_BED_WATER,
                every: 10,
                params: |t| {
                    let near = during(t, 5.0, 20.0) - during(t, 25.0, 40.0);
                    vec![
                        ease(5.0, 30.0, near),
                        ease(0.0, 12.0, near),
                        ease(0.15, 1.0, near),
                    ]
                },
                strikes: &[],
                expect: None,
            }],
        },
        Scene {
            name: "campfire",
            about: "Coming up to a fire that spreads, then leaving it",
            seconds: 45.0,
            tracks: vec![
                Track {
                    name: "fire",
                    kind: FOLEY_BED_FIRE,
                    every: 10,
                    params: |t| {
                        let near = during(t, 3.0, 12.0) - during(t, 32.0, 42.0);
                        vec![ease(1.0, 25.0, during(t, 12.0, 25.0)), ease(0.1, 1.0, near)]
                    },
                    strikes: &[],
                    expect: Some(NO_JUMP),
                },
                Track {
                    name: "lava",
                    kind: FOLEY_BED_LAVA,
                    every: 10,
                    params: |t| vec![60.0, ease(0.0, 0.4, during(t, 15.0, 30.0))],
                    strikes: &[],
                    expect: None,
                },
            ],
        },
        Scene {
            name: "long-woods",
            about: "Ten minutes in a breezy wood by a stream, for faults that come with time",
            seconds: 600.0,
            tracks: vec![
                Track {
                    name: "wind",
                    kind: FOLEY_BED_WIND,
                    every: 5,
                    params: |t| vec![4.0 + 2.0 * (t / 37.0).sin(), 0.6, 0.85],
                    strikes: &[],
                    expect: None,
                },
                Track {
                    name: "water",
                    kind: FOLEY_BED_WATER,
                    every: 10,
                    params: |t| vec![20.0, 2.0, 0.5 + 0.3 * (t / 53.0).sin()],
                    strikes: &[],
                    expect: None,
                },
            ],
        },
        Scene {
            name: "snaps",
            about: "Everything jumping in a tick, as on a teleport or a door: calm to gale, outside to in, near to far",
            seconds: 40.0,
            tracks: vec![
                Track {
                    name: "wind",
                    kind: FOLEY_BED_WIND,
                    every: 5,
                    params: |t| match (t / 5.0) as u32 % 4 {
                        0 => vec![1.0, 1.0, 0.9],
                        1 => vec![25.0, 1.0, 0.9],
                        2 => vec![25.0, 0.0, 0.0],
                        _ => vec![3.0, 1.0, 0.0],
                    },
                    strikes: &[],
                    expect: None,
                },
                Track {
                    name: "water",
                    kind: FOLEY_BED_WATER,
                    every: 10,
                    params: |t| match (t / 5.0) as u32 % 4 {
                        0 => vec![2.0, 0.0, 0.2],
                        1 => vec![100.0, 50.0, 1.0],
                        2 => vec![100.0, 50.0, 0.05],
                        _ => vec![0.0, 0.0, 0.0],
                    },
                    strikes: &[],
                    expect: None,
                },
                Track {
                    name: "rain",
                    kind: FOLEY_BED_RAIN,
                    every: 10,
                    params: |t| match (t / 5.0) as u32 % 4 {
                        0 => rain(0.0, 0.6, 0.3, None),
                        1 => rain(150.0, 0.6, 0.3, None),
                        2 => rain(150.0, 0.6, 0.3, Some((5.0, 3.0))),
                        _ => rain(5.0, 0.0, 0.0, None),
                    },
                    strikes: &[],
                    expect: None,
                },
            ],
        },
        // A bed is created because the world just got loud: a fire lit, a
        // pocket of lava mined open, a waterfall walked up to, a flash. These
        // start at that loud scene, not from silence, and nothing may click at
        // the onset.
        Scene {
            name: "cold-wind",
            about: "A bed made on a rooftop in a gale: wind starts at 20 m/s, not from still",
            seconds: 30.0,
            tracks: vec![Track {
                name: "wind",
                kind: FOLEY_BED_WIND,
                every: 5,
                params: |_| vec![20.0, 1.0, 0.9],
                strikes: &[],
                expect: Some(NO_STEP),
            }],
        },
        Scene {
            name: "cold-water",
            about: "A bed made at the lip of a fall: water starts at 40 falls, gain 1, not from silent",
            seconds: 20.0,
            tracks: vec![Track {
                name: "water",
                kind: FOLEY_BED_WATER,
                every: 10,
                params: |_| vec![0.0, 40.0, 1.0],
                strikes: &[],
                expect: Some(NO_STEP),
            }],
        },
        Scene {
            name: "cold-fire",
            about: "A bed made as a fire is lit: fire starts at 5 blocks, gain 1",
            seconds: 20.0,
            tracks: vec![Track {
                name: "fire",
                kind: FOLEY_BED_FIRE,
                every: 10,
                params: |_| vec![5.0, 1.0],
                strikes: &[],
                expect: Some(NO_STEP),
            }],
        },
        Scene {
            name: "cold-lava",
            about: "A bed made as lava is uncovered: lava starts at 4 blocks, gain 1",
            seconds: 20.0,
            tracks: vec![Track {
                name: "lava",
                kind: FOLEY_BED_LAVA,
                every: 10,
                params: |_| vec![4.0, 1.0],
                strikes: &[],
                expect: Some(NO_STEP),
            }],
        },
        Scene {
            name: "cold-rain",
            about: "A bed made in a shower already falling: rain starts at 40 mm/h",
            seconds: 20.0,
            tracks: vec![Track {
                name: "rain",
                kind: FOLEY_BED_RAIN,
                every: 10,
                params: |_| rain(40.0, 0.5, 0.2, None),
                strikes: &[],
                expect: Some(NO_STEP),
            }],
        },
        Scene {
            name: "cold-thunder",
            about: "A bed made by the flash itself: the first strike lands on a just-created thunder bed",
            seconds: 30.0,
            tracks: vec![Track {
                name: "thunder",
                kind: FOLEY_BED_THUNDER,
                every: 10,
                params: |_| vec![18.0, 95.0, 1.0],
                strikes: &[(0.5, 300.0, 0.0, 0.0)],
                expect: Some(NO_JUMP),
            }],
        },
        // Roofs, at the heights and of the stuff the game really meets: a
        // crawl, a low room, a tall hall; leaves over a wood, stone in a cave
        // mouth, metal over copper.
        Scene {
            name: "roof-heights",
            about: "Under one wooden roof as the ear drops from a hall to a crawl: height 16 to 0.5",
            seconds: 60.0,
            tracks: vec![Track {
                name: "rain",
                kind: FOLEY_BED_RAIN,
                every: 10,
                params: |t| rain(10.0, 0.5, 0.0, Some((3.0, 16.0 - 15.5 * during(t, 5.0, 55.0)))),
                strikes: &[],
                expect: Some(NO_STEP),
            }],
        },
        Scene {
            name: "wood-in-rain",
            about: "In a wood in the rain: canopy overhead, leaves the ground under it",
            seconds: 60.0,
            tracks: vec![Track {
                name: "rain",
                kind: FOLEY_BED_RAIN,
                every: 10,
                params: |_| rain(15.0, 0.05, 0.95, Some((2.0, 5.0))),
                strikes: &[],
                expect: Some(NO_STEP),
            }],
        },
        Scene {
            name: "roof-stuff",
            about: "The same rain under a leaf, stone, metal and soft roof, changing every 15 s",
            seconds: 60.0,
            tracks: vec![Track {
                name: "rain",
                kind: FOLEY_BED_RAIN,
                every: 10,
                params: |t| {
                    let roof = [(2.0, 4.0), (1.0, 3.0), (6.0, 2.0), (4.0, 3.0)][((t / 15.0) as usize) % 4];
                    rain(12.0, 0.4, 0.3, Some(roof))
                },
                strikes: &[],
                expect: Some(NO_STEP),
            }],
        },
        Scene {
            name: "downpour",
            about: "The loudest rain the game sends, then under a low roof: 50 mm/h open, then wood 2 up",
            seconds: 60.0,
            tracks: vec![Track {
                name: "rain",
                kind: FOLEY_BED_RAIN,
                every: 10,
                params: |t| {
                    let roof = (t > 30.0).then_some((3.0, 2.0));
                    rain(50.0, 0.5, 0.2, roof)
                },
                strikes: &[],
                expect: Some(NO_STEP),
            }],
        },
        // Thunder as the weather really gives it: the air's own temperature and
        // humidity, a strike close by, and the scene changing while a roll is
        // still in the air.
        Scene {
            name: "thunder-air",
            about: "A roll as the air cools and the rain stops: celsius 30 to 0, humidity 95 to 60, within the roll",
            seconds: 15.0,
            tracks: vec![Track {
                name: "thunder",
                kind: FOLEY_BED_THUNDER,
                every: 5,
                params: |t| vec![30.0 - 30.0 * during(t, 1.0, 12.0), 95.0 - 35.0 * during(t, 4.0, 11.0), 1.0],
                strikes: &[(2.0, 900.0, 0.0, 0.0)],
                expect: Some(NO_JUMP),
            }],
        },
        Scene {
            name: "thunder-doorway",
            about: "Struck in the open then shut in mid-roll, and out again: exposure 1 to 0 to 1 within the roll",
            seconds: 15.0,
            tracks: vec![Track {
                name: "thunder",
                kind: FOLEY_BED_THUNDER,
                every: 5,
                params: |t| vec![15.0, 90.0, (1.0 - during(t, 4.0, 5.0)) + during(t, 8.0, 9.0)],
                strikes: &[(2.0, 900.0, 0.0, 0.0)],
                expect: Some(NO_JUMP),
            }],
        },
        Scene {
            name: "thunder-near",
            about: "A bolt 50 m off, whose crack starts as sudden as it should: the level guard must keep it from clipping, not from cracking",
            seconds: 15.0,
            tracks: vec![Track {
                name: "thunder",
                kind: FOLEY_BED_THUNDER,
                every: 10,
                params: |_| vec![15.0, 90.0, 1.0],
                strikes: &[(2.0, 50.0, 0.0, 0.0)],
                // No clicks-or-steps check: a near crack's onset is sudden by
                // nature, and the guard ducking under it steps. What must hold
                // is the clip check every bed gets.
                expect: None,
            }],
        },
        // Wind above the weather's own: flying, gliding, a storm on a ridge.
        Scene {
            name: "flight",
            about: "An elytra ride up to 50 m/s and down, then into a wood at the leaves' own floor",
            seconds: 45.0,
            tracks: vec![Track {
                name: "wind",
                kind: FOLEY_BED_WIND,
                every: 5,
                params: |t| vec![
                    ease(8.0, 50.0, during(t, 2.0, 18.0)) - 45.0 * during(t, 25.0, 38.0),
                    ease(1.0, 0.2, during(t, 30.0, 40.0)),
                    ease(0.0, 0.9, during(t, 32.0, 42.0)),
                ],
                strikes: &[],
                expect: Some(NO_STEP),
            }],
        },
        // Water at the scale of a river or a lake edge: past the bubble cap at
        // flow 50 and the rush's flat top at 100, as the bank swings the gain.
        Scene {
            name: "bank-walk",
            about: "Walking a river bank: flow 20 to 300, falls 0 to 20, gain 0.08 to 1",
            seconds: 60.0,
            tracks: vec![Track {
                name: "water",
                kind: FOLEY_BED_WATER,
                every: 10,
                params: |t| vec![
                    ease(20.0, 300.0, during(t, 5.0, 30.0)) - 180.0 * during(t, 45.0, 58.0),
                    ease(0.0, 20.0, during(t, 10.0, 25.0)),
                    ease(0.08, 1.0, during(t, 5.0, 30.0)),
                ],
                strikes: &[],
                expect: Some(NO_STEP),
            }],
        },
        // Fire and lava at the sizes most met: a campfire, a couple of blocks,
        // a single lava block, and a lava shore stepping by tens of blocks.
        Scene {
            name: "one-campfire",
            about: "A campfire lit, spread to a few blocks, put out: fire 0.6, 1.6, 3.6, 1.6",
            seconds: 45.0,
            tracks: vec![Track {
                name: "fire",
                kind: FOLEY_BED_FIRE,
                every: 10,
                params: |t| {
                    let size = match (t / 10.0) as u32 {
                        0 => 0.6,
                        1 => 1.6,
                        2 => 3.6,
                        _ => 1.6,
                    };
                    vec![size, 1.0]
                },
                strikes: &[],
                expect: Some(NO_JUMP),
            }],
        },
        Scene {
            name: "lava-shore",
            about: "One block of lava opened up, then a lake edge: area 1 to 200, gain 1 to 0.6",
            seconds: 45.0,
            tracks: vec![Track {
                name: "lava",
                kind: FOLEY_BED_LAVA,
                every: 5,
                params: |t| vec![ease(1.0, 200.0, during(t, 5.0, 25.0)), ease(1.0, 0.6, during(t, 25.0, 40.0))],
                strikes: &[],
                expect: Some(NO_JUMP),
            }],
        },
        // Rain's ground, at the shares the world really gives: a glasshouse, a
        // copper roof, a lake, and the quietest real rain.
        Scene {
            name: "rain-grounds",
            about: "Rain's ground swept from a lake to stone to a glasshouse to metal, then a drizzle",
            seconds: 60.0,
            tracks: vec![Track {
                name: "rain",
                kind: FOLEY_BED_RAIN,
                every: 10,
                params: |t| match (t / 12.0) as u32 {
                    0 => rain_on(15.0, 0, None),
                    1 => rain_on(15.0, 1, None),
                    2 => rain_on(15.0, 5, None),
                    _ => rain_on(15.0, 6, None),
                },
                strikes: &[],
                expect: Some(NO_STEP),
            }],
        },
        Scene {
            name: "drizzle",
            about: "The quietest real rain: a light drizzle on grass, all soft",
            seconds: 30.0,
            tracks: vec![Track {
                name: "rain",
                kind: FOLEY_BED_RAIN,
                every: 10,
                params: |_| rain(1.0, 1.0, 0.0, None),
                strikes: &[],
                expect: Some(NO_STEP),
            }],
        },
        // Standing in it: rain, a campfire and a lava lake for minutes, for
        // faults that only show with time (the walks do this for wind, water).
        Scene {
            name: "long-weather",
            about: "Ten minutes standing in steady rain, a campfire beside and lava below",
            seconds: 600.0,
            tracks: vec![
                Track {
                    name: "rain",
                    kind: FOLEY_BED_RAIN,
                    every: 10,
                    params: |_| rain(10.0, 0.4, 0.3, None),
                    strikes: &[],
                    expect: Some(NO_STEP),
                },
                Track {
                    name: "fire",
                    kind: FOLEY_BED_FIRE,
                    every: 10,
                    params: |_| vec![0.6, 1.0],
                    strikes: &[],
                    expect: Some(NO_JUMP),
                },
                Track {
                    name: "lava",
                    kind: FOLEY_BED_LAVA,
                    every: 10,
                    params: |_| vec![6.0, 1.0],
                    strikes: &[],
                    expect: Some(NO_JUMP),
                },
            ],
        },
    ]
}
