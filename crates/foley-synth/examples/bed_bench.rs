//! What the beds cost: `cargo run --release -p foley-synth --example
//! bed_bench`. Prints a Markdown table, and appends it to the CI summary.

use std::time::Instant;

use foley_synth::{
    Fire, FireScene, Lava, LavaScene, Rain, RainScene, SAMPLE_RATE, Surface, Water, WaterScene,
    Wind, WindScene, strike,
};

/// Renders a bed's next samples.
type Renderer = Box<dyn FnMut(&mut [f32])>;

fn main() {
    let mut table = format!(
        "### Rain bed\n\n10 s of audio at {SAMPLE_RATE} Hz, on water, stone and leaves, {} {}.\n\n\
         | Rain rate | Time per second of audio | Share of a core |\n| --- | --- | --- |\n",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    for rate in [2.0, 10.0, 50.0] {
        let mut bed = Rain::new(1);
        let mut surfaces = [0.0; 7];
        for surface in [Surface::Water, Surface::Stone, Surface::Leaves] {
            surfaces[surface as usize] = 1.0 / 3.0;
        }
        bed.set_scene(RainScene {
            rate,
            surfaces,
            roof: None,
        });
        let mut out = vec![0.0f32; SAMPLE_RATE as usize];
        let start = Instant::now();
        for _ in 0..10 {
            bed.render(&mut out);
        }
        let per_second = start.elapsed().as_secs_f64() / 10.0;
        table += &format!(
            "| {rate} | {:.2} ms | {:.1} % |\n",
            per_second * 1e3,
            per_second * 100.0
        );
    }
    table += "\n### Wind bed\n\n| Wind speed | Time per second of audio | Share of a core |\n| --- | --- | --- |\n";
    for speed in [3.0, 10.0, 25.0] {
        let mut bed = Wind::new(1);
        bed.set_scene(WindScene {
            speed,
            exposure: 1.0,
            leaves: 0.5,
        });
        let mut out = vec![0.0f32; SAMPLE_RATE as usize];
        let start = Instant::now();
        for _ in 0..10 {
            bed.render(&mut out);
        }
        let per_second = start.elapsed().as_secs_f64() / 10.0;
        table += &format!(
            "| {speed} m/s | {:.2} ms | {:.1} % |\n",
            per_second * 1e3,
            per_second * 100.0
        );
    }
    table += "\n### Water, fire and lava beds\n\n| Bed | Time per second of audio | Share of a core |\n| --- | --- | --- |\n";
    let mut beds: Vec<(&str, Renderer)> = Vec::new();
    let mut stream = Water::new(1);
    stream.set_scene(WaterScene {
        flow: 20.0,
        falls: 0.0,
        gain: 1.0,
    });
    beds.push((
        "Stream, 20 blocks flowing",
        Box::new(move |out| stream.render(out)),
    ));
    let mut fall = Water::new(1);
    fall.set_scene(WaterScene {
        flow: 20.0,
        falls: 10.0,
        gain: 1.0,
    });
    beds.push((
        "Waterfall, 10 blocks landing",
        Box::new(move |out| fall.render(out)),
    ));
    let mut fire = Fire::new(1);
    fire.set_scene(FireScene {
        size: 20.0,
        gain: 1.0,
    });
    beds.push(("Fire, 20 blocks", Box::new(move |out| fire.render(out))));
    let mut lava = Lava::new(1);
    lava.set_scene(LavaScene {
        area: 200.0,
        gain: 1.0,
    });
    beds.push(("Lava, 200 blocks", Box::new(move |out| lava.render(out))));
    for (name, mut render) in beds {
        let mut out = vec![0.0f32; SAMPLE_RATE as usize];
        let start = Instant::now();
        for _ in 0..10 {
            render(&mut out);
        }
        let per_second = start.elapsed().as_secs_f64() / 10.0;
        table += &format!(
            "| {name} | {:.2} ms | {:.1} % |\n",
            per_second * 1e3,
            per_second * 100.0
        );
    }
    table += "\n### Thunder\n\nMade once a flash, on the stream thread.\n\n| Flash | Time to make it | Length |\n| --- | --- | --- |\n";
    for distance in [300.0, 4000.0] {
        let start = Instant::now();
        let sound = strike([distance, 0.0, 0.0], 1, 15.0, 90.0);
        table += &format!(
            "| {distance} m away | {:.1} ms | {:.1} s |\n",
            start.elapsed().as_secs_f64() * 1e3,
            sound.len() as f64 / f64::from(SAMPLE_RATE)
        );
    }
    print!("{table}");
    if let Ok(summary) = std::env::var("GITHUB_STEP_SUMMARY") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(summary)
            .expect("open the summary");
        writeln!(file, "{table}").expect("write the summary");
    }
}
