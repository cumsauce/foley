//! Every bed renders finite output at the rates a host might ask for.

use foley_dsp::{Cave, Fire, Lava, Rain, Underwater, Water, Wind};

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
