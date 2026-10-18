//! `Thunder::shutdown` must join the worker promptly and be idempotent;
//! the bed must render finite thunder at a non-default rate (Task 2 brief).

#[test]
fn thunder_worker_joins_on_shutdown() {
    let mut t = foley_synth::Thunder::new(7, 48_000);
    t.strike([300.0, 0.0, 0.0], 1);
    let started = std::time::Instant::now();
    t.shutdown();
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
    t.shutdown(); // idempotent no-op
}

#[test]
fn thunder_renders_strikes_at_44100() {
    let mut t = foley_synth::Thunder::new(9, 44_100);
    t.strike([300.0, 0.0, 0.0], 1);
    // Pump render() for ~2 s of frames (44.1 kHz), as the audio thread would.
    let mut frame = vec![0.0f32; 1024];
    let mut pumped = 0usize;
    let two_seconds = 2 * 44_100;
    while pumped < two_seconds {
        t.render(&mut frame);
        assert!(frame.iter().all(|s| s.is_finite()), "render must be finite");
        pumped += frame.len();
    }
}
