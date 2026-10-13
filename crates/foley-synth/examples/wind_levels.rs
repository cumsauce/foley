//! Wind levels across scenes: RMS and peak, for tuning.
use foley_synth::{Wind, WindScene};

fn main() {
    let scenes = [
        ("calm woods", 1.0, 0.3, 1.0),
        ("open field", 3.4, 1.0, 0.0),
        ("woods edge", 3.0, 0.8, 0.6),
        ("woods, rain", 2.0, 0.5, 1.0),
        ("hill top", 8.0, 1.0, 0.0),
        ("peak", 12.5, 1.0, 0.0),
        ("storm peak", 40.0, 1.0, 0.2),
        ("storm woods", 6.0, 0.6, 1.0),
    ];
    for (name, speed, exposure, leaves) in scenes {
        let mut wind = Wind::new(3);
        wind.set_scene(WindScene {
            speed,
            exposure,
            leaves,
        });
        let mut out = vec![0.0; 20 * 48_000];
        wind.render(&mut out);
        let tail = &out[48_000..];
        let rms = (tail.iter().map(|x| x * x).sum::<f32>() / tail.len() as f32).sqrt();
        let peak = tail.iter().fold(0f32, |m, x| m.max(x.abs()));
        println!(
            "{name:12} rms {:6.1} dB  peak {:6.1} dB",
            20.0 * rms.log10(),
            20.0 * peak.log10()
        );
    }
}
