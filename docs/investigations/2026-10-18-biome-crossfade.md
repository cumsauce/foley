# Biome switches and moving cut playback (hard cuts, no follow)

*2026-10-18. Root-caused in code and by measuring the rendered splice, not by
listening.*

## Symptom

> "there seems to be some cases where switching between biomes or simply moving
> causes stutters or abruptness in playback."

## The cut that is audible

A biome bed's kind ("desert", "deep dark", …) is a *choice* parameter: the FFI
lands it as a step inside the 128-sample glide
(`crates/foley-ffi/src/lib.rs`, `is_choice`, glide loop, `GLIDE_TIME = 0.3 s`).
Everything else glides; the kind does not.

Worse, `Biome::set_scene` (`crates/foley-synth/src/biome.rs`) rebuilt the whole
`BiomeState` on a kind change. Every running oscillator and envelope was cut
mid-tail, and the new state simply took over.

**Measured fail-before:** a bed that switched kind rendered **byte-identical** to
a bed that started fresh. The old biome's tail was gone at the splice — a hard
cut, up to `BED_CEILING = 0.4` of outgoing sample, exactly on the border tick.

## The companion cuts on the Java side

Same symptom, three places, and all three are the opposite of what the placed beds
already do:

- `audio/SurroundBeds.stop` stopped its four quarter beds **dead** with
  `SoundManager.stop`, while `audio/PlacedBeds` deliberately fades over ~3 s.
- Re-entering one of those biomes started four beds **at full volume** — a pop-in.
- `audio/CaveController.stop` hard-stopped the cave bed the same way.

## The "simply moving" bug

`CaveController` places the cave bed **once**, at the eye, and never repositions
it. Its *content* follows the player (the bed's parameters are driven by where
you are) but its *source* does not. Under Earshot, whose occlusion depends on
source position, the cave bed is progressively occluded as you walk away from
where it started — it fades and collapses toward a fixed point.

Compare `PlacedBeds.follow`, which re-places each slot every tick. That is the
pattern the cave bed was missing.

## Ruled out

- One-shot DSP on the audio/render thread — one-shots are rendered on
  `Util.backgroundExecutor()` and cached (`SoundBufferLibraryMixin`).
- Reallocation or locking in `bedRender` — scratch buffers are reused and the
  scene handoff is lock-free (`BedSound`).
- Sample-rate or channel changes mid-stream — `BedSound.FORMAT` is constant.
- Rain-roof stepping — its audible effect glides (`foley-dsp/src/rain.rs`).

## Fix

**Rust** (`crates/foley-synth/src/biome.rs`): a kind change now *crossfades*
rather than swapping state. The outgoing `BiomeState` is kept and rendered under
a `CROSSFADE = SAMPLE_RATE/8` (6000 samples ≈ 125 ms) ramp; the splice step is
now bounded by the stream's own maximum step, and the old biome stays audible
through the fade.

The crossfade only runs when the outgoing state was actually sounding
(`self.scene.gain > 0`). That gate was found the hard way: a fresh bed starts as
a gain-0 placeholder, and crossfading from it on the *first* kind set left
leftover desert hiss in the measurement window — which is also wrong product
behaviour. A newly started bed now gets its own kind at once.

**Java:**

- `BedSound` fades in (`heardWeight` starts at 0, raised by the existing
  0.05/tick weight glide ≈ 3 s) and gains a `fadeOut()` that retires the bed
  after ~60 ticks instead of vanishing — mirroring `PlacedBeds`' deliberate fade.
- `SurroundBeds.stop` and `CaveController.stop` fade their beds out instead of
  calling `SoundManager.stop`, so re-entry crossfades with the outgoing bed.
- `CaveController` now re-places the cave bed each tick with a
  `playsFrom.lerp(eye, 0.15)`, matching `PlacedBeds.follow`. **This is the
  "simply moving" fix.**

## Tests

- `biome::tests::a_kind_change_crossfades_rather_than_cutting` — fails before
  (switched bed byte-identical to a fresh one), passes after (switched ≠ fresh,
  splice step ≤ the stream's own max step, outgoing audible in the first 10 ms).
- `biome::tests::desert_hiss_is_higher_frequency_than_deep_dark_drone` — this
  caught the phantom-fade regression above, and is what proved the crossfade must
  be gated on the outgoing state actually sounding.
- `BedSoundTest.a_bed_fades_up_rather_than_starting_at_full_weight` and
  `fading_out_retires_the_bed_after_the_fade_not_at_once` — both fail before,
  pass after.

The stop-fade and the cave-bed follow need a live client to verify; they are
deliberately **not** covered by constant-asserting tests.

## Verified

`cargo test -p foley-synth --lib` → 107 passed, 0 failed (106 baseline + 1 new).
`cargo fmt --all -- --check` → clean.
`cd mod && ./gradlew cleanTest test` → **136 tests, 0 failures** (134 baseline +
2 new).
