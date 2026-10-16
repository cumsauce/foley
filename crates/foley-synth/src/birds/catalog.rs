//! The 50-species catalogue (M13): each `Birds::*` variant has a small
//! `Phrase` describing its call. Engine 2 vocabulary — [`crate::birds::engine2`]
//! — supplies the primitives. The implementations are descriptive, not
//! hand-tuned: the goal is for a plains to sound like a plains and a taiga
//! to sound like a taiga, every species from the real call of the bird that
//! lives there.

use crate::SAMPLE_RATE;
use crate::birds::Birds;
use crate::birds::engine2::{Phrase, Segment, SegmentKind};
use crate::dsp::normalize;

/// One phrase for the species, rendered from `seed`.
pub fn birdsong2(bird: Birds, seed: u64) -> Vec<f32> {
    let phrase = phrase_for(bird);
    let samples = phrase.render(seed);
    let mut samples = samples;
    samples.extend(std::iter::repeat_n(
        0.0,
        (0.05 * SAMPLE_RATE as f32) as usize,
    ));
    normalize(samples, 0.4)
}

fn phrase_for(bird: Birds) -> Phrase {
    match bird {
        Birds::BorealOwl => boreal_owl(),
        Birds::GreatGreyOwl => great_grey_owl(),
        Birds::CommonRaven => common_raven(),
        Birds::SpruceGrouse => spruce_grouse(),
        Birds::PineGrosbeak => pine_grosbeak(),
        Birds::BorealChickadee => boreal_chickadee(),
        Birds::VariedThrush => varied_thrush(),
        Birds::WesternMeadowlark => western_meadowlark(),
        Birds::HornedLark => horned_lark(),
        Birds::Dickcissel => dickcissel(),
        Birds::Bobolink => bobolink(),
        Birds::WoodThrush => wood_thrush(),
        Birds::HermitThrush => hermit_thrush(),
        Birds::ScarletTanager => scarlet_tanager(),
        Birds::RedEyedVireo => red_eyed_vireo(),
        Birds::KeelBilledToucan => keel_billed_toucan(),
        Birds::MontezumaOropendola => montezuma_oropendola(),
        Birds::ClayColoredThrush => clay_colored_thrush(),
        Birds::GreatTinamou => great_tinamou(),
        Birds::CrestedGuan => crested_guan(),
        Birds::LilacBreastedRoller => lilac_breasted_roller(),
        Birds::RedBilledHornbill => red_billed_hornbill(),
        Birds::KoriBustard => kori_bustard(),
        Birds::SouthernYellowBilledHornbill => southern_yellow_billed_hornbill(),
        Birds::CactusWren => cactus_wren(),
        Birds::CurveBilledThrasher => curve_billed_thrasher(),
        Birds::GreaterRoadrunner => greater_roadrunner(),
        Birds::Pyrrhuloxia => pyrrhuloxia(),
        Birds::GilaWoodpecker => gila_woodpecker(),
        Birds::Verdin => verdin(),
        Birds::SnowyOwl => snowy_owl(),
        Birds::SnowBunting => snow_bunting(),
        Birds::WillowPtarmigan => willow_ptarmigan(),
        Birds::Uguisu => uguisu(),
        Birds::Mejiro => mejiro(),
        Birds::VariedTit => varied_tit(),
        Birds::EurasianTreeSparrow => eurasian_tree_sparrow(),
        Birds::RedWingedBlackbird => red_winged_blackbird(),
        Birds::CommonYellowthroat => common_yellowthroat(),
        Birds::ProthonotaryWarbler => prothonotary_warbler(),
        Birds::Limpkin => limpkin(),
        Birds::ClarkNutcracker => clark_nutcracker(),
        Birds::GoldenEagle => golden_eagle(),
        Birds::AlpineChough => alpine_chough(),
        Birds::GreyCrownedRosyFinch => grey_crowned_rosy_finch(),
        Birds::CommonTern => common_tern(),
        Birds::Sanderling => sanderling(),
        Birds::MushroomTrill => mushroom_trill(),
    }
}

fn s(kind: SegmentKind, seconds: f32, gain: f32) -> Segment {
    Segment {
        kind,
        seconds,
        gain,
    }
}

fn hoot(hz: f32, sec: f32, g: f32) -> Segment {
    s(SegmentKind::Hoot { centre_hz: hz }, sec, g)
}

fn whistle(contour: Vec<(f32, f32)>, sec: f32, g: f32, h: f32) -> Segment {
    s(
        SegmentKind::Whistle {
            contour,
            attack: 0.02,
            release: 0.05,
            harmonic: h,
        },
        sec,
        g,
    )
}

fn fm(contour: Vec<(f32, f32)>, depth: f32, rate: f32, sec: f32, g: f32) -> Segment {
    s(
        SegmentKind::Fm {
            contour,
            depth_hz: depth,
            rate_hz: rate,
            attack: 0.04,
            release: 0.08,
        },
        sec,
        g,
    )
}

fn trill(base: f32, sweep: f32, am: f32, sec: f32, g: f32) -> Segment {
    s(
        SegmentKind::Trill {
            base_hz: base,
            sweep_hz: sweep,
            am_hz: am,
        },
        sec,
        g,
    )
}

fn croak(n: usize, base: f32, breath_db: f32, sec: f32, g: f32) -> Segment {
    s(
        SegmentKind::Croak {
            pulse_count: n,
            base_hz: base,
            breath_db,
        },
        sec,
        g,
    )
}

fn drum(rate: f32, dec: f32, sec: f32, g: f32) -> Segment {
    s(
        SegmentKind::Drum {
            rate_hz: rate,
            decay_s: dec,
        },
        sec,
        g,
    )
}

fn rattle(centre: f32, rate: f32, sec: f32, g: f32) -> Segment {
    s(
        SegmentKind::Rattle {
            centre_hz: centre,
            rate_hz: rate,
        },
        sec,
        g,
    )
}

// ----- Per-species phrases (M13 catalogue) -----

fn boreal_owl() -> Phrase {
    Phrase::new(vec![
        hoot(1000.0, 0.45, 0.7),
        hoot(1000.0, 0.30, 0.7),
        hoot(1050.0, 0.55, 0.95),
        hoot(1000.0, 0.30, 0.6),
    ])
}

fn great_grey_owl() -> Phrase {
    Phrase::new(vec![
        hoot(400.0, 0.6, 0.7),
        hoot(400.0, 0.7, 0.7),
        hoot(410.0, 0.9, 0.85),
    ])
}

fn common_raven() -> Phrase {
    Phrase::new(vec![
        croak(6, 800.0, -3.0, 1.2, 0.9),
        croak(4, 700.0, -3.0, 0.8, 0.7),
    ])
}

fn spruce_grouse() -> Phrase {
    Phrase::new(vec![drum(25.0, 1.0, 1.0, 0.8)])
}

fn pine_grosbeak() -> Phrase {
    Phrase::new(vec![
        whistle(vec![(0.0, 2000.0), (1.0, 2000.0)], 0.20, 0.6, 0.05),
        whistle(vec![(0.0, 2200.0), (1.0, 2200.0)], 0.20, 0.6, 0.05),
        whistle(vec![(0.0, 2400.0), (1.0, 2400.0)], 0.30, 0.7, 0.05),
    ])
}

fn boreal_chickadee() -> Phrase {
    Phrase::new(vec![
        whistle(vec![(0.0, 4500.0), (1.0, 4500.0)], 0.35, 0.85, 0.02),
        whistle(vec![(0.0, 3700.0), (1.0, 3700.0)], 0.32, 0.7, 0.02),
    ])
}

fn varied_thrush() -> Phrase {
    Phrase::new(vec![
        whistle(vec![(0.0, 3200.0), (1.0, 2800.0)], 0.30, 0.85, 0.4),
        whistle(vec![(0.0, 4200.0), (1.0, 3800.0)], 0.30, 0.85, 0.4),
    ])
}

fn western_meadowlark() -> Phrase {
    Phrase::new(vec![
        whistle(
            vec![(0.0, 2500.0), (0.5, 3200.0), (1.0, 2800.0)],
            0.30,
            0.85,
            0.05,
        ),
        trill(3000.0, 80.0, 7.0, 0.30, 0.7),
    ])
}

fn horned_lark() -> Phrase {
    Phrase::new(vec![
        whistle(vec![(0.0, 5500.0), (1.0, 6200.0)], 0.10, 0.6, 0.05),
        whistle(vec![(0.0, 6200.0), (1.0, 6800.0)], 0.10, 0.6, 0.05),
        whistle(vec![(0.0, 6000.0), (1.0, 5800.0)], 0.10, 0.6, 0.05),
    ])
}

fn dickcissel() -> Phrase {
    Phrase::new(vec![
        fm(vec![(0.0, 4200.0), (1.0, 4400.0)], 80.0, 12.0, 0.10, 0.75),
        fm(vec![(0.0, 4400.0), (1.0, 4200.0)], 80.0, 12.0, 0.10, 0.75),
        fm(
            vec![(0.0, 3000.0), (0.5, 3500.0), (1.0, 2800.0)],
            100.0,
            12.0,
            0.20,
            0.85,
        ),
    ])
}

fn bobolink() -> Phrase {
    Phrase::new(vec![
        rattle(4000.0, 60.0, 0.30, 0.7),
        rattle(3500.0, 50.0, 0.30, 0.7),
    ])
}

fn wood_thrush() -> Phrase {
    Phrase::new(vec![
        whistle(vec![(0.0, 2200.0), (1.0, 2900.0)], 0.20, 0.85, 0.03),
        trill(3200.0, 200.0, 10.0, 0.30, 0.85),
        whistle(vec![(0.0, 2500.0), (1.0, 1800.0)], 0.30, 0.85, 0.03),
    ])
}

fn hermit_thrush() -> Phrase {
    Phrase::new(vec![whistle(
        vec![(0.0, 3500.0), (0.5, 4500.0), (1.0, 2500.0)],
        1.4,
        0.9,
        0.05,
    )])
}

fn scarlet_tanager() -> Phrase {
    Phrase::new(vec![fm(
        vec![(0.0, 3000.0), (0.5, 3500.0), (1.0, 2800.0)],
        250.0,
        30.0,
        0.40,
        0.9,
    )])
}

fn red_eyed_vireo() -> Phrase {
    Phrase::new(vec![
        whistle(
            vec![(0.0, 2800.0), (0.5, 3500.0), (1.0, 3000.0)],
            0.18,
            0.7,
            0.05,
        ),
        whistle(
            vec![(0.0, 3200.0), (0.5, 4000.0), (1.0, 3500.0)],
            0.18,
            0.7,
            0.05,
        ),
        whistle(vec![(0.0, 3000.0), (1.0, 2800.0)], 0.15, 0.65, 0.05),
        whistle(vec![(0.0, 3500.0), (1.0, 3800.0)], 0.15, 0.6, 0.05),
    ])
}

fn keel_billed_toucan() -> Phrase {
    Phrase::new(vec![
        croak(8, 600.0, -6.0, 1.0, 0.85),
        croak(6, 600.0, -6.0, 0.7, 0.7),
    ])
}

fn montezuma_oropendola() -> Phrase {
    Phrase::new(vec![fm(
        vec![(0.0, 1500.0), (1.0, 800.0)],
        200.0,
        25.0,
        0.8,
        0.9,
    )])
}

fn clay_colored_thrush() -> Phrase {
    Phrase::new(vec![whistle(
        vec![(0.0, 1800.0), (0.3, 2200.0), (0.7, 2200.0), (1.0, 1800.0)],
        0.6,
        0.85,
        0.05,
    )])
}

fn great_tinamou() -> Phrase {
    Phrase::new(vec![whistle(
        vec![(0.0, 1200.0), (0.4, 1500.0), (0.7, 1500.0), (1.0, 1300.0)],
        1.5,
        0.85,
        0.03,
    )])
}

fn crested_guan() -> Phrase {
    Phrase::new(vec![croak(5, 350.0, -3.0, 1.0, 0.85)])
}

fn lilac_breasted_roller() -> Phrase {
    Phrase::new(vec![
        croak(3, 1200.0, -3.0, 0.30, 0.85),
        croak(3, 1100.0, -3.0, 0.30, 0.85),
    ])
}

fn red_billed_hornbill() -> Phrase {
    Phrase::new(vec![
        croak(4, 900.0, -6.0, 0.40, 0.85),
        croak(6, 950.0, -6.0, 0.50, 0.85),
    ])
}

fn kori_bustard() -> Phrase {
    Phrase::new(vec![hoot(250.0, 1.0, 0.9)])
}

fn southern_yellow_billed_hornbill() -> Phrase {
    Phrase::new(vec![croak(5, 1000.0, -8.0, 0.8, 0.85)])
}

fn cactus_wren() -> Phrase {
    Phrase::new(vec![rattle(3500.0, 20.0, 1.0, 0.85)])
}

fn curve_billed_thrasher() -> Phrase {
    Phrase::new(vec![
        whistle(vec![(0.0, 2200.0), (1.0, 2800.0)], 0.20, 0.85, 0.05),
        whistle(vec![(0.0, 2200.0), (1.0, 2800.0)], 0.20, 0.85, 0.05),
    ])
}

fn greater_roadrunner() -> Phrase {
    Phrase::new(vec![
        croak(5, 500.0, -4.0, 0.6, 0.85),
        drum(16.0, 0.4, 0.4, 0.7),
    ])
}

fn pyrrhuloxia() -> Phrase {
    Phrase::new(vec![
        whistle(vec![(0.0, 3500.0), (1.0, 3000.0)], 0.10, 0.7, 0.05),
        whistle(vec![(0.0, 3000.0), (1.0, 3500.0)], 0.08, 0.7, 0.05),
        whistle(vec![(0.0, 3200.0), (1.0, 3500.0)], 0.08, 0.7, 0.05),
        whistle(vec![(0.0, 3500.0), (1.0, 3300.0)], 0.10, 0.7, 0.05),
    ])
}

fn gila_woodpecker() -> Phrase {
    Phrase::new(vec![drum(30.0, 0.4, 0.4, 0.85)])
}

fn verdin() -> Phrase {
    Phrase::new(vec![
        whistle(vec![(0.0, 4500.0), (1.0, 4500.0)], 0.08, 0.6, 0.05),
        whistle(vec![(0.0, 4500.0), (1.0, 4500.0)], 0.08, 0.6, 0.05),
        whistle(vec![(0.0, 4800.0), (1.0, 4800.0)], 0.08, 0.6, 0.05),
    ])
}

fn snowy_owl() -> Phrase {
    Phrase::new(vec![
        hoot(450.0, 0.5, 0.85),
        hoot(450.0, 0.6, 0.85),
        hoot(480.0, 0.8, 0.95),
    ])
}

fn snow_bunting() -> Phrase {
    Phrase::new(vec![rattle(5000.0, 22.0, 0.4, 0.85)])
}

fn willow_ptarmigan() -> Phrase {
    Phrase::new(vec![
        croak(5, 700.0, -6.0, 0.6, 0.85),
        rattle(3000.0, 18.0, 0.3, 0.7),
    ])
}

fn uguisu() -> Phrase {
    Phrase::new(vec![
        fm(vec![(0.0, 1000.0), (1.0, 1000.0)], 60.0, 6.0, 0.7, 0.85),
        trill(1200.0, 80.0, 12.0, 0.7, 0.85),
    ])
}

fn mejiro() -> Phrase {
    Phrase::new(vec![
        whistle(vec![(0.0, 4500.0), (1.0, 4500.0)], 0.07, 0.6, 0.05),
        whistle(vec![(0.0, 4500.0), (1.0, 4500.0)], 0.07, 0.6, 0.05),
        whistle(vec![(0.0, 4500.0), (1.0, 4500.0)], 0.07, 0.6, 0.05),
    ])
}

fn varied_tit() -> Phrase {
    Phrase::new(vec![
        whistle(vec![(0.0, 3800.0), (1.0, 3800.0)], 0.05, 0.6, 0.05),
        whistle(vec![(0.0, 3800.0), (1.0, 3800.0)], 0.05, 0.6, 0.05),
        whistle(vec![(0.0, 3800.0), (1.0, 3800.0)], 0.05, 0.6, 0.05),
        whistle(vec![(0.0, 3800.0), (1.0, 3800.0)], 0.05, 0.6, 0.05),
    ])
}

fn eurasian_tree_sparrow() -> Phrase {
    Phrase::new(vec![whistle(
        vec![(0.0, 3200.0), (0.5, 3800.0), (1.0, 3000.0)],
        0.30,
        0.7,
        0.1,
    )])
}

fn red_winged_blackbird() -> Phrase {
    Phrase::new(vec![
        fm(
            vec![(0.0, 2500.0), (0.5, 3000.0), (1.0, 2500.0)],
            100.0,
            12.0,
            0.20,
            0.85,
        ),
        trill(3500.0, 200.0, 8.0, 0.35, 0.85),
    ])
}

fn common_yellowthroat() -> Phrase {
    Phrase::new(vec![trill(3500.0, 200.0, 10.0, 0.7, 0.85)])
}

fn prothonotary_warbler() -> Phrase {
    Phrase::new(vec![whistle(
        vec![(0.0, 3500.0), (1.0, 4500.0)],
        0.50,
        0.85,
        0.05,
    )])
}

fn limpkin() -> Phrase {
    Phrase::new(vec![fm(
        vec![(0.0, 950.0), (1.0, 950.0)],
        50.0,
        5.0,
        1.5,
        0.85,
    )])
}

fn clark_nutcracker() -> Phrase {
    Phrase::new(vec![rattle(1800.0, 12.0, 1.0, 0.85)])
}

fn golden_eagle() -> Phrase {
    Phrase::new(vec![whistle(
        vec![(0.0, 4500.0), (0.4, 5500.0), (1.0, 4000.0)],
        1.0,
        0.85,
        0.05,
    )])
}

fn alpine_chough() -> Phrase {
    Phrase::new(vec![
        whistle(
            vec![(0.0, 3500.0), (0.5, 3800.0), (1.0, 3500.0)],
            0.10,
            0.6,
            0.05,
        ),
        whistle(
            vec![(0.0, 3500.0), (0.5, 3800.0), (1.0, 3500.0)],
            0.10,
            0.6,
            0.05,
        ),
    ])
}

fn grey_crowned_rosy_finch() -> Phrase {
    Phrase::new(vec![trill(2500.0, 100.0, 8.0, 0.4, 0.85)])
}

fn common_tern() -> Phrase {
    Phrase::new(vec![whistle(
        vec![(0.0, 3500.0), (1.0, 4500.0)],
        0.30,
        0.85,
        0.05,
    )])
}

fn sanderling() -> Phrase {
    Phrase::new(vec![trill(1500.0, 80.0, 12.0, 0.20, 0.7)])
}

fn mushroom_trill() -> Phrase {
    Phrase::new(vec![fm(
        vec![(0.0, 400.0), (0.5, 500.0), (1.0, 400.0)],
        30.0,
        12.0,
        1.5,
        0.85,
    )])
}
