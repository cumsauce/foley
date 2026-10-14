//! The scripted scenes: what each bed is told as the world goes by, as the
//! game's controllers would tell it.

use foley_ffi::{
    FOLEY_BED_FIRE, FOLEY_BED_LAVA, FOLEY_BED_RAIN, FOLEY_BED_THUNDER, FOLEY_BED_WATER,
    FOLEY_BED_WIND, FOLEY_SURFACES,
};

/// A bed through a scene: its kind, how often (ticks) the game sets its
/// scene, its params at a time (seconds), and when lightning strikes it.
pub struct Track {
    pub name: &'static str,
    pub kind: i32,
    pub every: u32,
    pub params: fn(f32) -> Vec<f32>,
    pub strikes: &'static [f32],
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
                        rain(rate, 0.6, 0.3, roof)
                    },
                    strikes: &[],
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
                },
                Track {
                    name: "thunder",
                    kind: FOLEY_BED_THUNDER,
                    every: 10,
                    params: |t| vec![18.0, 85.0, if t > 50.0 { 0.1 } else { 1.0 }],
                    strikes: &[25.0, 38.0, 55.0],
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
                },
                Track {
                    name: "lava",
                    kind: FOLEY_BED_LAVA,
                    every: 10,
                    params: |t| vec![60.0, ease(0.0, 0.4, during(t, 15.0, 30.0))],
                    strikes: &[],
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
                },
                Track {
                    name: "water",
                    kind: FOLEY_BED_WATER,
                    every: 10,
                    params: |t| vec![20.0, 2.0, 0.5 + 0.3 * (t / 53.0).sin()],
                    strikes: &[],
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
                },
            ],
        },
    ]
}
