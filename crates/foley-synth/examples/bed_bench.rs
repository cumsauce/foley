//! What the rain bed costs: `cargo run --release -p foley-synth --example
//! bed_bench`. Prints a Markdown table, and appends it to the CI summary.

use std::time::Instant;

use foley_synth::{RainOnWater, SAMPLE_RATE};

fn main() {
    let mut table = format!(
        "### Rain on water bed\n\n10 s of audio at {SAMPLE_RATE} Hz, {} {}.\n\n\
         | Drops a second | Time per second of audio | Share of a core |\n| --- | --- | --- |\n",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    for rate in [100.0, 400.0, 1500.0, 5000.0] {
        let mut bed = RainOnWater::new(1, rate);
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
