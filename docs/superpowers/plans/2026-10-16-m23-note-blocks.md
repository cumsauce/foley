# M23 — Note Block Instruments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Foley plays its own sound for every Minecraft note block: 16 instruments × 25 notes, each modelled after the block it sits on (a harp is a plucked string, a basedrum a membrane, a bell a struck bowl) and synthesised at the exact pitch, replacing vanilla's `block.note_block.<instrument>` events.

**Architecture:** One new Rust primitive, `FOLEY_PRIMITIVE_NOTE`, renders any (instrument, note) on demand. Internally it dispatches to five synthesis families — pluck (Karplus-Strong), modal (struck bars and bells), membrane (drums), breath (tube), electronic — each a small function. Java detects the vanilla note-block event in `Replacements`, reads the instrument from the event name and the note from the note block's block state, and plays a runtime-chosen generated buffer name via `AddonSounds.instance(...)`, which resolves a generated path **without any sounds.json entry** (documented at `AddonSounds.java:231-238`). No per-note sounds.json entries exist.

**Tech Stack:** Rust (`foley-synth`, `foley-ffi`), Java 21 (mod, Fabric 1.21.x), cbindgen (`cargo xtask header`).

**Spec:** `docs/PLAN.md` (this plan is the first draft of a new M23 section; the instrument list is authoritative from `SoundEvents.NOTE_BLOCK_*`).

## Global Constraints

- Bump `FOLEY_ABI_VERSION` from **23 to 24** in `crates/foley-ffi/src/lib.rs`, the regenerated `crates/foley-ffi/include/foley.h`, and the Java mirror `NativeBridge.ABI_VERSION`. Add the changelog line in the existing style (`/// 24: note block instruments (M23): FOLEY_PRIMITIVE_NOTE.`).
- **One new constant only**: `FOLEY_PRIMITIVE_NOTE: i32 = 5` (next free primitive id). Java mirror `NativeBridge.PRIMITIVE_NOTE = 5`. Do NOT add 16 shot kinds and do NOT add any sounds.json entries.
- The primitive takes **3 floats**: `[instrument, note, character]` — `instrument` 0..15 (int-valued), `note` 0..24 (int-valued, vanilla's note block note), `character` 0..1 (a timbre/decay knob; the Java side passes a per-hit variation so successive hits are not bit-identical). Validated with the existing `range(...)` helper; `FOLEY_PRIMITIVE_MAX_FLOATS` already allows 4.
- **Audio path invariant**: rendering must not allocate on the audio thread. Note renders are one-shots built on the game's buffer-library thread (like every other `generated/` model at `GeneratedSounds.render`), so a `Vec` per render is correct here; do NOT use the streaming beds' no-alloc pool. The existing `every_primitive_renders_bounded_sound` test (`primitives.rs:111`) must cover the new primitive.
- **Pitch is synthesised, not resampled**: the buffer is rendered at the note's own frequency and played at pitch `1.0`. The Java side passes the note (0..24), never a playback-rate.
- The instrument index mapping is Foley's own and lives in one place (a Rust table and its Java twin must agree — see Task 7's Interface).
- Toggles: one new `FoleyConfig` boolean `noteBlocks`, default **on**, gating the whole feature (the standing rule: every sound Foley replaces can be given back to the game). Config convention: the boolean sits after `sculk` (the M21 block), before `caveAtmosphere`.
- The vanilla instruments are enumerated from `SoundEvents.NOTE_BLOCK_*` in the mappings — do not transcribe them from memory. Task 1 pins the list and its order; every later task consumes that order.

## Review Focus

- **An unknown or non-note-block sound must never be swallowed.** The detection matches `block.note_block.` ids only, and an unrecognised instrument returns "let vanilla play" — not "cancel and silence". (Pinned in Task 8 by `unknownNoteSoundIsLeftToVanilla`.)
- **A note block with no block beneath it, or with a note Foley cannot read, still sounds.** Fall back to deriving the note from the sound instance's own pitch rather than dropping the hit. (Pinned in Task 8 by `unreadableNoteFallsBackToTheInstancePitch`.)
- **Every instrument actually makes a sound and stays bounded.** Twenty-five notes × sixteen instruments, all finite, all peak ≤ 1, none silent, none ending on a click. (Pinned in Task 6 by `every_instrument_every_note_renders`.)
- **Pitch accuracy across the whole range.** A note block's note 0..24 must render at half to double the instrument's base, within a few percent — the point of modelling rather than resampling. (Pinned per family in Tasks 2–5 with a measured fundamental.)
- **The instrument is the block's, not the player's.** Two note blocks with different blocks beneath them, struck identically, must render different families. (Pinned in Task 1 by `the_seventeen_instruments_map_to_the_expected_families`.)

---

## Task 1: The note module, the instrument table, and the pluck family (TDD)

**Files:**
- Create: `crates/foley-synth/src/note.rs`
- Modify: `crates/foley-synth/src/lib.rs` (module + re-export)
- Test: inline `#[cfg(test)]` in `note.rs`

**Interfaces:**

```rust
/// Which family models an instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family { Pluck, Modal, Membrane, Breath, Electronic }

/// The vanilla note block instruments, in Foley's index order (0..=15),
/// each with the family that models it and the base frequency (Hz) of its
/// note 12. The order is Foley's own; the Java twin is `NoteBlocks.INSTRUMENTS`.
pub fn family(instrument: u32) -> Option<Family>;
pub fn base_hz(instrument: u32) -> Option<f32>;

/// The note's own frequency: the instrument's base times 2^((note-12)/12).
pub fn note_hz(instrument: u32, note: f32) -> Option<f32>;

/// One note block hit, rendered at its own pitch. `character` (0 to 1)
/// varies the timbre so successive hits differ; the seed mixes the same.
pub fn note(instrument: u32, note: f32, character: f32, seed: u64) -> Option<Vec<f32>>;
```

Index order (0..=15), which every later task consumes: `0 harp, 1 bass, 2 basedrum, 3 snare, 4 hat, 5 bell, 6 flute, 7 chime, 8 guitar, 9 xylophone, 10 iron_xylophone, 11 cow_bell, 12 didgeridoo, 13 bit, 14 banjo, 15 pling`. This is the vanilla `NoteBlockInstrument` constant order as `SoundEvents.NOTE_BLOCK_*` declares it — **confirm the exact order from the mappings before writing the table** (grep the loom/mappings for `NOTE_BLOCK_`), and if the mapped order differs, use the mapped order and say so in your report.

Families: Pluck = harp, bass, guitar, banjo. Modal = bell, chime, xylophone, iron_xylophone, cow_bell. Membrane = basedrum, snare, hat. Breath = flute, didgeridoo. Electronic = bit, pling.

- [ ] **Step 1: Write failing tests in `crates/foley-synth/src/note.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_seventeen_instruments_map_to_the_expected_families() {
        assert_eq!(family(0), Some(Family::Pluck));       // harp
        assert_eq!(family(2), Some(Family::Membrane));    // basedrum
        assert_eq!(family(5), Some(Family::Modal));       // bell
        assert_eq!(family(6), Some(Family::Breath));      // flute
        assert_eq!(family(13), Some(Family::Electronic)); // bit
        assert_eq!(family(16), None);                     // out of range
    }
    #[test]
    fn a_note_octave_above_the_base_doubles_the_frequency() {
        let base = base_hz(0).unwrap();
        assert!((note_hz(0, 12.0).unwrap() - base).abs() < 1e-3);
        assert!((note_hz(0, 24.0).unwrap() - 2.0 * base).abs() < 1e-3);
        assert!((note_hz(0, 0.0).unwrap() - 0.5 * base).abs() < 1e-3);
    }
    #[test]
    fn an_unknown_instrument_renders_nothing() {
        assert!(note(16, 12.0, 0.5, 1).is_none());
    }
}
```

- [ ] **Step 2: Run; expect compile error on missing `Family`/`family`/`note`**

Run: `cargo test -p foley-synth note`
Expected: compile error naming the missing items.

- [ ] **Step 3: Implement the table, `note_hz`, and the Pluck family**

`note` dispatches on `family(instrument)`; for `Family::Pluck` it calls the Karplus-Strong model written in this task (`pluck(hz, damping, brightness, seed) -> Vec<f32>`): a delay line of `SAMPLE_RATE / hz` samples, seeded with a short burst of lowpass-filtered noise (the pluck), fed back through a one-pole lowpass (the string's damping) with the loop gain chosen so the string decays over `character`-scaled seconds. Length ≈ 3 s or until −60 dB, whichever comes first. Normalise with `crate::dsp::normalize_in_place(&mut out, 0.9)` as the other primitives do, and per instrument vary damping, brightness and pluck position (harp clean, bass long and dull, banjo bright and snappy, guitar between). `crate::dsp::filter`'s `LowPass`/`BandPass` and `Rng` are available.

- [ ] **Step 4: Run tests; expect pass**

Run: `cargo test -p foley-synth note`
Expected: 3 tests pass.

- [ ] **Step 5: Re-export from `crates/foley-synth/src/lib.rs`**

Add `pub mod note;` and `pub use note::{Family, family, base_hz, note, note_hz};` beside the existing re-exports.

- [ ] **Step 6: Commit**

```bash
git add crates/foley-synth/src/note.rs crates/foley-synth/src/lib.rs
git commit -m "feat(synth): note block instrument table and the plucked-string family"
```

---

## Task 2: The modal family — struck bars and bells (TDD)

**Files:**
- Modify: `crates/foley-synth/src/note.rs`
- Test: inline `#[cfg(test)]` in `note.rs`

**Interfaces:**
- Consumes: `Family`, `note_hz` (Task 1).
- Produces: `fn modal(hz: f32, inharmonicity: f32, damping: f32, seed: u64) -> Vec<f32>` used by `note` for `Family::Modal`.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn a_bar_rings_at_the_note_it_was_struck_at() {
    let out = modal(440.0, 0.2, 0.5, 1);
    assert!((fundamental(&out) - 440.0).abs() < 40.0, "bar at {}", fundamental(&out));
}
#[test]
fn an_inharmonic_bar_has_partials_off_the_harmonic_series() {
    // The 2nd partial of a metal bar is not 2x the fundamental.
    let wood = modal(440.0, 0.0, 0.5, 2);
    let metal = modal(440.0, 0.6, 0.5, 2);
    assert!(partial_ratio(&metal, &wood) > 1.1);
}
#[test]
fn higher_damping_rings_longer() {
    let short = modal(440.0, 0.3, 0.1, 3);
    let long = modal(440.0, 0.3, 0.9, 3);
    assert!(tail_rms(&long) > 2.0 * tail_rms(&short));
}
```

(`fundamental`, `partial_ratio`, `tail_rms` are small local helpers — zero-crossing rate on the loudest window, ratio of the strongest non-DC spectral peak positions, RMS of the back half — write them in the test module; Task 1's file already has a zero-crossing helper to mirror.)

- [ ] **Step 2: Run; expect failure/compile error on `modal`**

Run: `cargo test -p foley-synth note`
Expected: compile error naming `modal`.

- [ ] **Step 3: Implement `modal`**

A bank of decaying sinusoids: a fundamental at `hz` plus partials at `hz * ratio_i` where the ratios interpolate from a near-harmonic bar (ratios ≈ 1, 2.76, 5.40, 8.93 for a free-free bar) toward an inharmonic bell as `inharmonicity` rises, each partial with its own decay (higher partials die faster), scaled by `damping`. Reuse `bell.rs`'s approach (three-mode additive with inharmonic ratios at `bell.rs:44-49`) as the shape, generalised. Instrument voicings: bell and chime long and inharmonic, xylophone short and near-harmonic, iron_xylophone metallic and long, cow_bell clanky and short.

- [ ] **Step 4: Run tests; expect pass, then commit**

Run: `cargo test -p foley-synth note`
Expected: 6 tests pass (3 from Task 1, 3 new).

```bash
git add crates/foley-synth/src/note.rs
git commit -m "feat(synth): struck bars and bells for the note block modal family"
```

---

## Task 3: The membrane family — drums (TDD)

**Files:**
- Modify: `crates/foley-synth/src/note.rs`
- Test: inline `#[cfg(test)]` in `note.rs`

**Interfaces:**
- Consumes: `Family`, `note_hz`.
- Produces: `fn membrane(hz: f32, drop: f32, noise_mix: f32, seed: u64) -> Vec<f32>` for `Family::Membrane`.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn a_drum_starts_at_its_note_and_falls() {
    let out = membrane(200.0, 0.6, 0.2, 1);
    assert!(fundamental_at(&out, 0.0) > fundamental_at(&out, 0.3), "the pitch must drop");
}
#[test]
fn a_snare_is_noisier_than_a_basedrum() {
    let drum = membrane(150.0, 0.6, 0.0, 2);
    let snare = membrane(150.0, 0.4, 1.0, 2);
    assert!(roughness(&snare) > roughness(&drum));
}
#[test]
fn a_hat_is_short_and_bright() {
    let hat = membrane(9000.0, 0.0, 1.0, 3);
    assert!(hat.len() < crate::SAMPLE_RATE as usize / 2);
    assert!(centroid(&hat) > centroid(&membrane(150.0, 0.6, 0.0, 3)));
}
```

- [ ] **Step 2: Run; expect failure on `membrane`**
- [ ] **Step 3: Implement `membrane`** — a decaying sine whose frequency starts at `hz` and glides down by `drop` (the skin's tension release) with a short click at onset, mixed with band-passed noise (`noise_mix`) for the snare's rattle and the hat's air; the hat is entirely noise with a very short envelope, basedrum almost wholly tone.
- [ ] **Step 4: Run tests; expect pass, then commit**

```bash
git add crates/foley-synth/src/note.rs
git commit -m "feat(synth): drums for the note block membrane family"
```

---

## Task 4: The breath family — flute and didgeridoo (TDD)

**Files:**
- Modify: `crates/foley-synth/src/note.rs`
- Test: inline `#[cfg(test)]` in `note.rs`

**Interfaces:**
- Consumes: `Family`, `note_hz`.
- Produces: `fn breath(hz: f32, airflow: f32, seed: u64) -> Vec<f32>` for `Family::Breath`.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn a_flute_sounds_the_note_it_is_asked_for() {
    let out = breath(587.0, 0.3, 1);
    assert!((fundamental(&out) - 587.0).abs() < 40.0, "flute at {}", fundamental(&out));
}
#[test]
fn a_didgeridoo_is_richer_than_a_flute() {
    assert!(harmonic_count(&breath(150.0, 1.0, 2)) > harmonic_count(&breath(587.0, 0.2, 2)));
}
#[test]
fn the_breath_noise_rides_under_the_tone() {
    let quiet = breath(440.0, 0.0, 3);
    let windy = breath(440.0, 1.0, 3);
    assert!(high_band_energy(&windy) > high_band_energy(&quiet));
}
```

- [ ] **Step 2: Run; expect failure on `breath`**
- [ ] **Step 3: Implement `breath`** — a resonant tube driven by a mixture of a glottal-ish pulse (for the didgeridoo's buzz, rich in harmonics) and breath noise (for the flute's air), the mix set by `airflow`; the tube's resonances track `hz`. Reuse `crate::voice::tract`'s waveguide if it is usable standalone (check its public API first — `crates/foley-synth/src/voice/tract.rs`); if it is not, a simpler two-formant resonator at `hz` and `hz*3` is acceptable — say which you used and why.
- [ ] **Step 4: Run tests; expect pass, then commit**

```bash
git add crates/foley-synth/src/note.rs
git commit -m "feat(synth): flute and didgeridoo for the note block breath family"
```

---

## Task 5: The electronic family — bit and pling (TDD)

**Files:**
- Modify: `crates/foley-synth/src/note.rs`
- Test: inline `#[cfg(test)]` in `note.rs`

**Interfaces:**
- Consumes: `Family`, `note_hz`.
- Produces: `fn electronic(hz: f32, character: f32, seed: u64) -> Vec<f32>` for `Family::Electronic`.

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn a_bit_is_a_square_wave_at_the_note() {
    let out = electronic(440.0, 0.0, 1); // bit
    assert!((fundamental(&out) - 440.0).abs() < 30.0);
}
#[test]
fn a_pling_is_an_fm_pair_that_decays() {
    let out = electronic(440.0, 1.0, 2); // pling
    assert!(out.len() < 3 * crate::SAMPLE_RATE as usize);
    assert!(tail_rms(&out) < 0.2 * head_rms(&out), "the pling must decay");
}
#[test]
fn the_two_electronics_differ() {
    assert_ne!(electronic(440.0, 0.0, 3).len(), electronic(440.0, 1.0, 3).len());
}
```

- [ ] **Step 2: Run; expect failure on `electronic`**
- [ ] **Step 3: Implement `electronic`** — `character < 0.5`: a square (bit) wave at `hz` with a short percussive envelope and a touch of vibrato/pitch drop; `character >= 0.5`: a two-operator FM "pling" (sine carrier at `hz`, sine modulator at ~`hz*3.5`, modulating index falling from ~2 to 0 over a few hundred ms) — the classic electric-piano strike.
- [ ] **Step 4: Run tests; expect pass, then commit**

```bash
git add crates/foley-synth/src/note.rs
git commit -m "feat(synth): bit and pling for the note block electronic family"
```

---

## Task 6: `FOLEY_PRIMITIVE_NOTE` in foley-ffi (TDD)

**Files:**
- Modify: `crates/foley-ffi/src/lib.rs`
- Modify: `crates/foley-ffi/include/foley.h` (regenerated)
- Test: `crates/foley-ffi/src/lib.rs` (the primitive test loop + a new note case)

**Interfaces:**
- Consumes: `foley_synth::note(instrument, note, character, seed)`.
- Produces: `FOLEY_PRIMITIVE_NOTE = 5`, ABI 24.

- [ ] **Step 1: Wire the primitive and write the failing test**

Add the constant after `FOLEY_PRIMITIVE_AEOLIAN`:

```rust
/// Note block instruments (M23): `instrument` (0 to 15), `note` (0 to 24,
/// vanilla's note block note), `character` (0 to 1, a per-hit timbre).
pub const FOLEY_PRIMITIVE_NOTE: i32 = 5;
```

Add the dispatch arm in `foley_render_primitive`'s `match kind` (beside `FOLEY_PRIMITIVE_AEOLIAN` at `lib.rs:1084`):

```rust
FOLEY_PRIMITIVE_NOTE => foley_synth::note(
    range("instrument", given(0)?, 0.0, 15.0)?.round() as u32,
    range("note", given(1)?, 0.0, 24.0)?,
    gain(2)?,
    seed,
)
.ok_or_else(|| FfiError::invalid("unknown instrument"))?,
```

Add a test asserting every instrument × every note (16 × 25) renders non-silent, finite, bounded PCM through the primitive, and that `instrument = 99` returns an error.

- [ ] **Step 2: Run; expect the new test to fail (missing constant/arm)**

Run: `cargo test -p foley-ffi note`
Expected: compile error, then the new assertion failing before the arm is added.

- [ ] **Step 3: Bump the ABI and document it**

`FOLEY_ABI_VERSION` → `24`, with `/// 24: note block instruments (M23): `FOLEY_PRIMITIVE_NOTE`.`

- [ ] **Step 4: Regenerate the header**

Run: `cargo xtask header`, then `cargo xtask header --check` → exit 0. Confirm the diff adds only the constant, the doc lines and the ABI number — no signature change.

- [ ] **Step 5: Run ffi tests; expect pass**

Run: `cargo test -p foley-ffi`
Expected: all pass (19 baseline + new).

- [ ] **Step 6: Commit**

```bash
git add crates/foley-ffi/src/lib.rs crates/foley-ffi/include/foley.h
git commit -m "feat(ffi): add FOLEY_PRIMITIVE_NOTE and bump ABI to 24"
```

---

## Task 7: Java — the instrument table, the buffer name, and the render (TDD)

**Files:**
- Create: `mod/src/main/java/io/github/cumsauce/foley/audio/NoteBlocks.java`
- Modify: `mod/src/main/java/io/github/cumsauce/foley/audio/GeneratedSounds.java`
- Modify: `mod/src/main/java/io/github/cumsauce/foley/natives/NativeBridge.java` (`PRIMITIVE_NOTE = 5`, `ABI_VERSION = 24`)
- Test: `mod/src/test/java/io/github/cumsauce/foley/audio/NoteBlocksTest.java`

**Interfaces:**
- Consumes: `NativeBridge.PRIMITIVE_NOTE`, the existing `bridge.renderPrimitive(...)` (mirror `renderOneShot`).
- Produces: `NoteBlocks.EVENT` (a `SoundEvent`, naming only), `NoteBlocks.INSTRUMENTS` (a `List<String>` in the same order as the Rust table), `NoteBlocks.instrument(Identifier) -> int` (−1 if not ours), `NoteBlocks.bufferName(int instrument, int note, int variant) -> Identifier`, `NoteBlocks.noteAt(Minecraft, double x, double y, double z) -> int` (−1 if that block is not a note block), `NoteBlocks.noteOf(SoundInstance, Minecraft) -> int` (the note, from the block state or the instance's pitch), and the `note_` arm in `GeneratedSounds`.

- [ ] **Step 1: Write failing tests**

```java
@Test
void theInstrumentOrderMatchesTheIds() {
    assertEquals(16, NoteBlocks.INSTRUMENTS.size());
    assertEquals("harp", NoteBlocks.INSTRUMENTS.get(0));
    assertEquals("pling", NoteBlocks.INSTRUMENTS.get(15));
}
@Test
void aVanillaNoteBlockIdYieldsItsInstrument() {
    assertEquals(0, NoteBlocks.instrument(Identifier.fromNamespaceAndPath("minecraft", "block.note_block.harp")));
    assertEquals(-1, NoteBlocks.instrument(Identifier.fromNamespaceAndPath("minecraft", "block.note_block.unknown")));
    assertEquals(-1, NoteBlocks.instrument(Identifier.fromNamespaceAndPath("minecraft", "block.stone.break")));
}
@Test
void theBufferNameEncodesTheInstrumentAndNote() {
    assertEquals("foley:generated/note_0_12_0", NoteBlocks.bufferName(0, 12, 0).toString());
}
```

- [ ] **Step 2: Run; expect compile error on missing `NoteBlocks`**

Run: `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests '*NoteBlocksTest*'`

- [ ] **Step 3: Implement `NoteBlocks`** — the `INSTRUMENTS` list, `instrument(Identifier)` parsing `block.note_block.<name>` (returning −1 when the name is not in the list), `bufferName(instrument, note, variant)` → `foley:generated/note_<instrument>_<note>_<variant>`, and the note-reading helper `noteAt(Minecraft, double x, double y, double z)` returning the note block's `NoteBlock.NOTE` property or −1 if the block there is not a note block.
- [ ] **Step 4: Add the `note_` arm to `GeneratedSounds.life`** — parse `note_<instrument>_<note>_<variant>` (the existing `life` split on the last `_` gives the variant) and call `bridge.renderPrimitive(NativeBridge.PRIMITIVE_NOTE, new float[] {instrument, note, character(seed)}, seed)`; the `character` maps the seed to 0..1. Follow the `case "surf" ->` shape at `GeneratedSounds.java:58`.
- [ ] **Step 5: Bump `NativeBridge`** — `PRIMITIVE_NOTE = 5` beside the other primitives, `ABI_VERSION = 24`.
- [ ] **Step 6: Run tests; expect pass, then commit**

```bash
git add mod/src/main/java/io/github/cumsauce/foley/audio/NoteBlocks.java mod/src/main/java/io/github/cumsauce/foley/audio/GeneratedSounds.java mod/src/main/java/io/github/cumsauce/foley/natives/NativeBridge.java mod/src/test/java/io/github/cumsauce/foley/audio/NoteBlocksTest.java
git commit -m "feat(audio): note block instrument table and generated note buffer"
```

---

## Task 8: Java — replace the vanilla note block sounds (TDD)

**Files:**
- Modify: `mod/src/main/java/io/github/cumsauce/foley/audio/NoteBlocks.java` (the play path)
- Modify: `mod/src/main/java/io/github/cumsauce/foley/audio/Replacements.java` (one arm)
- Modify: `mod/src/main/java/io/github/cumsauce/foley/config/FoleyConfig.java` (`noteBlocks`, default on)
- Modify: `mod/src/main/java/io/github/cumsauce/foley/ui/FoleyScreen.java` (one row)
- Modify: `mod/src/test/java/io/github/cumsauce/foley/config/FoleyConfigTest.java` (round-trip)
- Test: `mod/src/test/java/io/github/cumsauce/foley/audio/NoteBlocksTest.java`

**Interfaces:**
- Consumes: everything in Task 7; `AddonSounds.instance(...)` (`AddonSounds.java:235`); `FoleyClient.wants(FoleyConfig::noteBlocks)`.
- Produces: `NoteBlocks.replace(SoundInstance, Minecraft) -> boolean`.

- [ ] **Step 1: Write failing tests**

```java
@Test
void unknownNoteSoundIsLeftToVanilla() {
    assertFalse(NoteBlocks.replace(sound("block.note_block.unknown"), client()));
}
@Test
void aNoteBlockSoundIsTaken() {
    assertTrue(NoteBlocks.replace(sound("block.note_block.harp"), client()));
}
@Test
void unreadableNoteFallsBackToTheInstancePitch() {
    assertEquals(12, NoteBlocks.noteOf(soundWithPitch("block.note_block.harp", 1.0f), null));
}
```

- [ ] **Step 2: Run; expect failure**

Run: `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests '*NoteBlocksTest*'`

- [ ] **Step 3: Implement the play path** — `NoteBlocks.replace` returns false unless the id is a note block id Foley knows; when it is and the toggle is on, read the note from the block state at the sound (falling back to `12 + 12*log2(pitch)` rounded when the block cannot be read — the fall-back documented in the Review Focus), pick a variant from the sound's seed hash, and play `AddonSounds.instance(NoteBlocks.EVENT, NoteBlocks.bufferName(instrument, note, variant), SoundSource.RECORDS, volume * FoleyClient.gain(), 1f, x, y, z)`; return true. `NoteBlocks.EVENT` is a Foley `SoundEvent` (naming only; `Notes.event(...)` or `Replacements.event(...)` per the file's helper).
- [ ] **Step 4: Add the `Replacements` arm** — call `NoteBlocks.replace(sound, minecraft)` early in `replace` (before the ambience block), returning its result; keep the toggle inside `NoteBlocks.replace`.
- [ ] **Step 5: Add the config field** — `noteBlocks` after `sculk`, default `true`, with its wither, `fromJson`/`toJson` lines, and the `DEFAULT`/`ALL`/`NONE` wiring (the pattern is 34 booleans today → 35). Add a `FoleyScreen` row beside the M21 toggles.
- [ ] **Step 6: Run the full mod suite; expect pass, then commit**

Run: `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test build --no-configuration-cache`
Expected: BUILD SUCCESSFUL; the config round-trip and the new note tests green.

```bash
git add mod/src/main/java/io/github/cumsauce/foley/audio/NoteBlocks.java mod/src/main/java/io/github/cumsauce/foley/audio/Replacements.java mod/src/main/java/io/github/cumsauce/foley/config/FoleyConfig.java mod/src/main/java/io/github/cumsauce/foley/ui/FoleyScreen.java mod/src/test/java/io/github/cumsauce/foley/audio/NoteBlocksTest.java mod/src/test/java/io/github/cumsauce/foley/config/FoleyConfigTest.java
git commit -m "feat(audio): play Foley's own note block sounds"
```

---

## Task 9: Gametest, docs and the version bump

**Files:**
- Modify: `mod/src/gametest/java/io/github/cumsauce/foley/gametest/FoleyClientGameTest.java`
- Modify: `docs/PLAN.md` (a new `## M23 results` section)
- Modify: `Cargo.toml`, `Cargo.lock`, `mod/gradle.properties`

**Interfaces:**
- Consumes: everything above.

- [ ] **Step 1: Add the gametest step**

Place a note block with a known block beneath it (e.g. gold → bell) and strike it, asserting Foley's event fired (mirror the M21/M17 steps at `FoleyClientGameTest.java:387`/`:417`); then toggle `noteBlocks` off and assert vanilla plays instead.

- [ ] **Step 2: Write the M23 results section** — the five families and which instruments they cover, the primitive, ABI 24, the note reading and its fall-back, the toggle, and the tests, in the document's voice.

- [ ] **Step 3: Bump the version** `0.1.0-beta.57` → `0.1.0-beta.58` in `Cargo.toml`, every `Cargo.lock` entry, and `mod/gradle.properties`; confirm `fabric.mod.json` still uses `${version}`. Verify: `grep -rn "0\.1\.0-beta\.57" . | grep -v "/target/" | grep -v "/build/" | grep -v "\.git/"` → no source hits.

- [ ] **Step 4: Full verification**

`cargo test --workspace --lib && cargo fmt --all --check && cargo clippy --workspace --all-targets --locked -- -D warnings` then `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test build --no-configuration-cache`.

- [ ] **Step 5: Commit**

```bash
git add mod/src/gametest/java/io/github/cumsauce/foley/gametest/FoleyClientGameTest.java docs/PLAN.md Cargo.toml Cargo.lock mod/gradle.properties
git commit -m "v0.1.0-beta.58: note block instruments (M23)"
```
