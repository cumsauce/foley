//! `warm_voice_tables` forces the voice table `OnceLock` eagerly and is
//! idempotent (Task 2 brief).

#[test]
fn warm_voice_tables_is_idempotent() {
    foley_synth::warm_voice_tables();
    foley_synth::warm_voice_tables(); // second call must not panic or rebuild
}
