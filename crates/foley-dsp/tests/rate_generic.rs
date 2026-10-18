//! Every bed renders finite output at the rates a host might ask for.

use foley_dsp::{Cave, CaveScene, Fire, Lava, Rain, Underwater, Water, Wind, WindScene};

fn finite(buf: &[f32]) -> bool {
    buf.iter().all(|s| s.is_finite())
}

macro_rules! render_at {
    // One case per bed, so each bed is independently auditable.
    ($name:ident, $ctor:expr) => {
        #[test]
        fn $name() {
            for rate in [44_100u32, 96_000, 192_000] {
                let mut bed = $ctor(rate);
                let mut out = vec![0.0f32; 512];
                bed.render(&mut out);
                assert!(finite(&out), "non-finite at {rate} Hz");
            }
        }
    };
}

render_at!(rain_44100_96k_192k, |rate| Rain::new(1234, rate));
render_at!(water_44100_96k_192k, |rate| Water::new(1234, rate));
render_at!(fire_44100_96k_192k, |rate| Fire::new(1234, rate));
render_at!(lava_44100_96k_192k, |rate| Lava::new(1234, rate));
render_at!(cave_44100_96k_192k, |rate| Cave::new(1234, rate));
render_at!(underwater_44100_96k_192k, |rate| Underwater::new(
    1234, rate
));
render_at!(wind_44100_96k_192k, |rate| Wind::new(1234, rate));

/// The warm-up run inside `set_scene` is rate-scaled (see `warm_up` on each
/// bed), so a fresh bed settles for the same time in seconds at any rate and
/// the first heard samples stay finite.
#[test]
fn set_scene_then_render_is_finite_at_every_rate() {
    for rate in [44_100u32, 96_000, 192_000] {
        let mut wind = Wind::new(1234, rate);
        wind.set_scene(WindScene {
            speed: 8.0,
            exposure: 1.0,
            leaves: 1.0,
        });
        let mut out = vec![0.0f32; 512];
        wind.render(&mut out);
        assert!(finite(&out), "wind non-finite after set_scene at {rate} Hz");

        let mut cave = Cave::new(1234, rate);
        cave.set_scene(CaveScene {
            depth: 0.7,
            mass: 0.8,
            openness: 0.2,
            drips: 1.0,
            gain: 1.0,
        });
        let mut out = vec![0.0f32; 512];
        cave.render(&mut out);
        assert!(finite(&out), "cave non-finite after set_scene at {rate} Hz");
    }
}
