//! Every one-shot family must render finite audio at non-default rates
//! (Task 2: foley-synth rate-generic refactor, brief Step 1).

use foley_synth::{Call, Contact, Material, Mob, Plant};

#[test]
fn oneshots_finite_at_nondefault_rates() {
    fn check(name: &str, make: impl Fn(u32) -> Vec<f32>) {
        let at_default = make(foley_synth::DEFAULT_SAMPLE_RATE);
        for rate in [44_100u32, 96_000u32] {
            let out = make(rate);
            assert!(
                out.iter().all(|s| s.is_finite()),
                "{name} at {rate} must be finite"
            );
            if !at_default.is_empty() {
                assert!(!out.is_empty(), "{name} at {rate} must be non-empty");
            }
        }
    }

    // The four families.
    check("contact", |r| {
        foley_synth::contact(Contact::Tap, Material::Wood, 6.0, 1, r)
    });
    check("fracture", |r| foley_synth::fracture(Material::Wood, 2, r));
    check("note", |r| {
        foley_synth::note(0, 60.0, 0.5, 3, r).unwrap_or_default()
    });
    check("voice", |r| {
        foley_synth::voice(
            Mob::from_index(0).expect("a mob"),
            Call::from_index(0).expect("a call"),
            4,
            r,
        )
    });

    // Each of the 19 named shots (FOLEY_SHOT_* order).
    check("drip_into_water", |r| {
        foley_synth::drip_into_water(0.003, 5, r)
    });
    check("drip_on_stone", |r| foley_synth::drip_on_stone(0.003, 6, r));
    check("lava_drip", |r| foley_synth::lava_drip(7, r));
    check("crickets", |r| foley_synth::crickets(25.0, 8, r));
    check("birdsong", |r| foley_synth::birdsong(0, 9, r));
    check("frog", |r| foley_synth::frog(5.0, 10, r));
    check("surf", |r| foley_synth::surf(1.0, 11, r));
    check("woodpecker", |r| foley_synth::woodpecker(12, r));
    check("owl", |r| foley_synth::owl(13, r));
    check("cicadas", |r| foley_synth::cicadas(30.0, 14, r));
    check("howl", |r| foley_synth::howl(15, r));
    check("brush", |r| foley_synth::brush(Plant::of(0), 16, r));
    check("ice_singing", |r| foley_synth::ice_singing(17, r));
    check("settling_stone", |r| foley_synth::settling_stone(18, r));
    check("flush", |r| foley_synth::flush(19, r));
    check("buzz", |r| foley_synth::buzz(2, 20, r));
    check("sculk_ring", |r| foley_synth::sculk_ring(1.0, 21, r));
    check("bell", |r| foley_synth::bell(10.0, 22, r));
    check("bang", |r| foley_synth::bang(1.0, 23, r));
}

#[test]
fn surf_len_scales_with_rate() {
    assert!(foley_synth::surf_len(96_000) > foley_synth::surf_len(44_100));
    assert_eq!(
        foley_synth::SURF_LEN,
        foley_synth::surf_len(foley_synth::DEFAULT_SAMPLE_RATE)
    );
}
