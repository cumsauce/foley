//! What the rain bed costs: `cargo run --release -p foley-synth --example
//! bed_bench`. Prints a Markdown table, and appends it to the CI summary.

use std::time::Instant;

use foley_synth::{Rain, RainScene, SAMPLE_RATE, Surface};

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
