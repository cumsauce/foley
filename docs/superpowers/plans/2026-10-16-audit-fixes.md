# Foley audit fixes — implementation plan

Source: read-only audit of `ab629e7` (merge of M17). Every finding below was
verified by reading the code at the line quoted. No finding is speculative.

## Context

The Foley mod synthesises ambient audio in Rust and streams it into Minecraft
through a C ABI. The audit found bugs in the FFI glide/render path, one
allocation/robustness hole, several Java controller defects (the known
"threshold chatter" class), and count/doc drift. The work is split into three
independent file-disjoint tasks so they can be implemented and reviewed in
parallel, plus one documentation task.

## Global constraints (binding — a change that breaks one is rejected)

1. **The audio render path must never allocate.** Enforced by
   `the_audio_thread_never_allocates` in `crates/foley-ffi/src/lib.rs` using
   the counting allocator in `crates/foley-ffi/src/no_alloc.rs`. No new
   `Vec`/`Box`/`String`/`collect` on any render path.
2. **`foley_bed_render` must fully define `out` (zero it) on every error path**
   — bed error, render error, and panic. Do not weaken this.
3. **Scene float order and count must match exactly** across the Rust `apply`
   arms, the `wanted` float-count match, the `foley.h` doc comments, and the
   Java `Scene.params()` order.
4. **ABI stability**: `FOLEY_ABI_VERSION = 23`; bed ids 0–9; shot ids 0–17.
   Do not renumber. Any change to a doc comment that propagates into
   `crates/foley-ffi/include/foley.h` must go through `cargo xtask header`
   (CI runs `cargo xtask header --check`).
   `NativeBridge.ABI_VERSION` (Java) mirrors the Rust constant.
5. **CI gates**: `cargo fmt --all --check` and
   `clippy --workspace --all-targets --locked -- -D warnings`. Do not touch the
   deliberate `#![allow(dead_code, clippy::type_complexity)]` first line of
   `crates/foley-synth/src/birds/engine1.rs`.
6. **Env**: prepend `$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin`
   to `PATH`; `CARGO_BUILD_JOBS=1` is mandatory. Java:
   `JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew … --no-configuration-cache`
   run from `mod/`. A `rust-objcopy`/`libLLVM.dylib` SIGABRT warning is a
   broken toolchain install, not a code bug — ignore it.

---

## Task 1: Rust — biome kind snapping, render-path robustness, FFI test hardening

File: `crates/foley-ffi/src/lib.rs`,
`crates/foley-synth/src/biome.rs`, `crates/foley-synth/src/surf.rs`.

### 1a. `Biome` `kind` must be a choice, not a glide (Bug, audible)

**Location**: `crates/foley-ffi/src/lib.rs:593-597` (`is_choice`), used at
`:660`; `crates/foley-synth/src/biome.rs:170-185` (`set_scene`), docs at
`biome.rs:1-8` and `:58-64`.

**Root cause**: `is_choice` only snaps rain:

```rust
fn is_choice(synth: &Synth, index: usize) -> bool {
    matches!(synth, Synth::Rain(_)) && index > FOLEY_SURFACES
}
```

So `Synth::Biome` param 0 (`kind`, `p[0].round() as i64` in the Biome arm at
`lib.rs:552-575`) is glided over `GLIDE_TIME = 0.3 s`. A biome change
Desert(0) → Snow(5) re-`apply`s heard kinds 1, 2, 3, 4, and `Biome::set_scene`
rebuilds `self.state` on every different kind — four wrong biome beds for
~60 ms each. `BiomeController` keeps ONE bed and calls `setScene` on it
(`audio/SurroundBeds.java:52-63`, `audio/BiomeController.java:35`), so this
fires on every biome boundary crossing. The `biome.rs` module doc claims the
opposite ("the synth itself does not switch kinds mid-stream").

**Change**:

```rust
fn is_choice(synth: &Synth, index: usize) -> bool {
    (matches!(synth, Synth::Rain(_)) && index > FOLEY_SURFACES)
        || (matches!(synth, Synth::Biome(_)) && index == 0)
}
```

Update the doc comment above it to say: "a rain bed's roof surface and room
size, and a biome bed's kind." Rewrite the two stale sentences in
`crates/foley-synth/src/biome.rs` (module doc ~lines 1-8 and the `set_scene`
doc ~lines 58-64) to: "Java keeps one bed and changes its scene; `kind` is
snapped by the FFI's glide, so the state is rebuilt once per biome change."
Also fix the `sub` doc in `biome.rs` — it currently says "mangrove leaf count
/ 16"; the truth (Java `BiomeController`, `lib.rs` Biome arm) is the raw leaf
count, 0..=200.

**Test** (add to `mod tests` in `lib.rs`, next to `every_bed_renders_pcm`):
create a BIOME bed, render 512 frames at Desert `[0.0, 1.0, 0.0, 1.0]`, then
`foley_bed_set_scene` to Snow `[5.0, 1.0, 0.0, 1.0]`, render 128 frames, and
assert `(*bed).heard[0] == 5.0` (it snaps on the first block; with the bug it
would be ~0.044). The test module has `use super::*`, so `FoleyBed` and its
`heard` field are in scope.

**Dependencies**: none. Contains no `foley.h`-propagating doc comment change
(verify with `cargo xtask header --check`).

### 1b. Non-finite samples must not permanently silence a bed (Bug, robustness)

**Location**: `crates/foley-ffi/src/lib.rs:299-302` (`UnderwaterLpf::process`),
used at `:694`; the per-sample loop at `:687-696`.

**Root cause**: `self.y += self.a * (x - self.y); mix * self.y + (1.0 - mix) * x`.
One non-finite `x` from any synth makes `y` NaN forever, and `0.0 * NaN` is
still NaN, so even at `mix == 0.0` every later sample is NaN →
`to_pcm` → 0 → the bed is silent until Java recreates it. `bed.envelope` and
`bed.guard` inherit the poison too (`:689`, `:690`). Nothing on the render path
sanitises, and this low-pass lacks the subnormal `flush()` guard its sibling in
`crates/foley-dsp/src/filter.rs:72-75` has.

**Change**:

In the per-sample loop in `render_bed`, sanitise the input sample:

```rust
for (out, sample) in out.iter_mut().zip(block.iter()) {
    let sample = if sample.is_finite() { *sample } else { 0.0 };
    bed.envelope = sample.abs().max(bed.envelope * 0.99996);
    …
    let filtered = bed.underwater.process(sample, bed.underwater_mix);
```

and in `UnderwaterLpf::process`:

```rust
fn process(&mut self, x: f32, mix: f32) -> f32 {
    self.y += self.a * (x - self.y);
    if self.y.abs() < 1e-20 {
        self.y = 0.0;
    }
    mix * self.y + (1.0 - mix) * x
}
```

**Test**: unit-test `UnderwaterLpf` directly: one `f32::NAN` input at
`mix = 0.0`, then `1.0` at `mix = 0.0` — the second output must be exactly
`1.0` and finite.

**Dependencies**: none. Must not change the no-allocation property (both edits
are pure arithmetic).

### 1c. Arm RealtimeSanitizer in the allocation test (Test bug — CI gate is a no-op)

**Location**: `crates/foley-ffi/src/lib.rs:1216-1311`
(`the_audio_thread_never_allocates`), doc comment `:1205-1215`, canary
`:1195-1206`.

**Root cause**: the test's closure ends with `rtsan_standalone::realtime_exit()`
but there is **no** `realtime_enter()` anywhere in the crate except the
`#[ignore]` canary. CI's `RTSAN_ENABLE=1 cargo test -p foley-ffi audio_thread`
step therefore detects nothing; only the allocation counter is live, while the
doc comment claims RealtimeSanitizer "also watches those calls".

**Change**: the closure body is, per scene iteration:
`foley_bed_set_scene`; `foley_bed_strike` (thunder only, first two scenes);
`for _ in 0..40 { foley_bed_render }`; `rtsan_standalone::realtime_exit()`.
Insert `rtsan_standalone::realtime_enter();` as the first statement after the
`foley_bed_strike` block and immediately before the `for _ in 0..40` render
loop, and keep the trailing `realtime_exit()`. Do **not** wrap
`foley_bed_create` — it is outside the closure and spawns thunder's worker
thread.

Contingency: the same closure's return value is asserted `== 0` allocations,
so if `realtime_enter()` itself allocates, the test fails with a count that is
not about the render path. If that happens, restructure so the counter still
measures the render path, e.g. arm RTSan around the whole closure and keep the
allocation count on a nested `allocations(|| { … })` that excludes
`realtime_enter`/`realtime_exit` — and report which structure you used. If
RTSan instead reports a lock, sleep or syscall anywhere else in the render
path, that is a **real finding**: report the failure verbatim, leave the
assertion in place, and do not weaken or remove it. Note that `set_scene` and
`strike` are deliberately left outside the realtime region for now; widening
the region to cover them is a separate change.

**Dependencies**: do after 1a/1b so the newly-armed sanitiser covers them.

### 1d. Unify the hard-coded bed lists and add BIOME (Test gap)

**Location**: `crates/foley-ffi/src/lib.rs:1335-1365` (`every_bed_renders_pcm`),
`:1216-1311` (`the_audio_thread_never_allocates`).

**Root cause**: `every_bed_renders_pcm` lists 8 beds — it omits
`FOLEY_BED_BIOME` and `FOLEY_BED_THUNDER`. THUNDER is silent without a strike
(defensible, but say so in a comment); BIOME is a real gap. Both tests keep
their own copy of the bed list, which is how BIOME went missing.

**Change**:
1. Add `(FOLEY_BED_BIOME, &[0.0f32, 1.0, 0.0, 1.0][..])` to the
   `every_bed_renders_pcm` loop, and a comment recording why THUNDER is absent
   ("silent until struck; covered by `thunder_is_struck`").
2. Add a module-level test const `const ALL_BEDS: &[(i32, &[f32])]` listing all
   ten beds with a scene that renders non-zero, using exactly the values
   `every_bed_renders_pcm` already uses for the eight it lists, plus
   `(FOLEY_BED_BIOME, &[0.0f32, 1.0, 0.0, 1.0][..])` and
   `(FOLEY_BED_THUNDER, &[20.0f32, 70.0, 1.0][..])` (the thunder scene from
   `the_audio_thread_never_allocates`). Add
   `debug_assert_eq!(ALL_BEDS.len(), (FOLEY_BED_SURF + 1) as usize);`.
   Have `every_bed_renders_pcm` iterate `ALL_BEDS` and assert each renders a
   non-zero sample, skipping THUNDER (silent until struck).
3. In `the_audio_thread_never_allocates`, add a comment above its `beds` array:
   `// keep in sync with ALL_BEDS` — do not restructure that array (it drives
   multi-scene glide coverage).

**Dependencies**: after 1c (same test function).

### 1e. Small cleanups in `lib.rs` / `surf.rs`

**Location**: `crates/foley-ffi/src/lib.rs:848-858`
(`foley_render_one_shot`), test comment near `:1216-1311`
("0.25 s at a time"), `crates/foley-synth/src/surf.rs:162-187` (spawn-cost
comment "≈11 ms per 0.25 s pull").

**Root cause**: two consecutive match arms in `foley_render_one_shot` both
return `Err("size out of range")` (one covers `FOLEY_SHOT_SCULK_RING |
FOLEY_SHOT_BELL`, the next duplicates the same error); and the "0.25 s" figure
is wrong — `BedSound.Stream.read` caps at `MOST_BYTES = 48000 & ~1` bytes =
24 000 frames = **0.5 s** at 48 kHz mono 16-bit.

**Change**: merge the duplicate arm into the one above it; correct both
comments to "0.5 s"; change the test's per-render buffer from 12 000 to 24 000
frames so it matches the real pull size.

**Dependencies**: none.

### Task 1 verification

```
export PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"
export CARGO_BUILD_JOBS=1
cargo fmt --all
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test -p foley-ffi --lib
cargo xtask header --check
RTSAN_ENABLE=1 cargo test --locked -p foley-ffi audio_thread
```

Expected: fmt clean; clippy no warnings; all foley-ffi lib tests pass; header
fresh (regenerate with `cargo xtask header` and include `foley.h` in the
commit only if the check fails); the RTSan run passes (or its failure is
reported verbatim per 1c). Report the exact output of each command.

```
[acceptance]
criteria: cargo test -p foley-ffi --lib passes, cargo clippy --workspace --all-targets --locked -- -D warnings is clean, cargo fmt --all --check is clean, cargo xtask header --check is clean, and the implementer report quotes the raw command output for each
deliverable: crates/foley-ffi/src/lib.rs, crates/foley-synth/src/biome.rs, crates/foley-synth/src/surf.rs
[/acceptance]
```

---

## Task 2: Java — surf controller linger and hygiene, plus two small audio fixes

Files: `mod/src/main/java/io/github/cumsauce/foley/audio/SurfController.java`,
`…/WindScene.java`, `…/BedSound.java`, `…/Life.java`, and
`mod/src/test/java/io/github/cumsauce/foley/audio/SurfControllerTest.java`.

### 2a. `SurfController` flaps the bed on/off (Bug — threshold chatter)

**Location**: `SurfController.java:63-68` and `:93-94` (`scene.height() <= 0`
→ `stop()`), `:150-171` (`findShore`).

**Root cause**: `findShore` probes 24 random columns inside a 32-block box and
returns `null` on a miss. A miss sets `SurfScene.SILENT`, whose `height()` is
0, so `tick` calls `stop()`. The next sample that hits builds a **new**
`BedSound` — new native seed, new stream, cold surf state. On a thin shoreline
the per-sample hit rate is well under 1, so the bed restarts every few
seconds. The same randomness also makes `near` (line 96-98) jump between
samples, because each sample may pick a different shore column.

**Change**:
1. Extract the per-column test from the loop body (lines 154-168) into
   `private boolean isShore(ClientLevel level, int x, int z, double eyeY)`,
   returning true when the column top is water source and a land neighbour
   qualifies — same logic, same constants, no behaviour change.
2. Add fields `private @Nullable BlockPos lastShore;` and `private int misses;`.
3. At the top of `findShore`, if `lastShore != null` and it is within `RADIUS`
   horizontally of `eye`, re-test it with `isShore`; on success return it
   (`misses = 0`). This makes the reported shore — and therefore `near` —
   stable between samples.
4. After the random tries, on a hit set `lastShore` and `misses = 0`; on a
   miss return `null` as today.
5. In `sample`, when `findShore` returns `null`:
   `if (lastShore != null && ++misses < MISSES_TO_STOP) return scene;` — keep
   the previous scene for a few samples instead of flapping — otherwise
   `lastShore = null; misses = 0; return SurfScene.SILENT;`. Declare
   `private static final int MISSES_TO_STOP = 3;` beside `RADIUS`, with a
   comment mirroring the cave/wind dry-linger rationale.

**Dependencies**: none.

### 2b. Salt/fresh tier read at the listener, not the shore (Bug)

**Location**: `SurfController.java:99-101`.

**Root cause**: `Holder<Biome> biome = level.getBiome(BlockPos.containing(eye));`
— standing on a plains bank beside an ocean classifies as fresh water
(`heightFor` → 0.3 m "lake lapping" for an ocean), and a shore of a pond beside
a beach classifies as sea.

**Change**: read the biome at the shore column already found:
`level.getBiome(shore)`. Nothing else changes.

**Dependencies**: 2a (same method region).

### 2c. `SurfController` re-derives the wind bed's storm mix (Refactor — drift risk)

**Location**: `SurfController.java:103-106` duplicates
`WindScene.java:52` (`float reference = CLEAR + (RAIN - CLEAR) * level.getRainLevel(1f) + (STORM - RAIN) * level.getThunderLevel(1f);`).

**Change**: add to `WindScene`:

```java
/** The m/s a 10 m reference wind would have in this weather. */
static float reference(ClientLevel level) {
    return CLEAR + (RAIN - CLEAR) * level.getRainLevel(1f) + (STORM - RAIN) * level.getThunderLevel(1f);
}

/** That reference expressed as 0..1 between {@link #CLEAR} and {@link #STORM}. */
static float storm(ClientLevel level) {
    return Math.clamp((reference(level) - CLEAR) / (STORM - CLEAR), 0f, 1f);
}
```

`WindScene.sample` uses `reference(level)`; `SurfController.sample` replaces
its inline rain/thunder expression with `WindScene.storm(level)` and keeps the
existing `Math.clamp(…, 0f, 1f)` result. Both must produce identical numbers:
`storm` is the same expression SurfController's inline code already computes.

**Dependencies**: 2a/2b (same file).

### 2d. Delete `SurfController.surgeRate` (Refactor — two sources of truth)

**Location**: `SurfController.java:133-141`, tests at
`SurfControllerTest.java:27-34`.

**Root cause**: it duplicates the Rust bed's cadence
(`crates/foley-synth/src/surf.rs:152-155`, `12.0 - 10.0 * clamp(storm*exposure)`)
in a second language and is used only by its own unit test.

**Change**: delete `surgeRate` and the two tests that pin it
(`surgeRateRisesWithStormAndWind`, and whichever test asserts `1/12` / `0.5`).
Leave `heightFor`, `exposureFor` and their tests — those are used by the
controller.

**Dependencies**: none, but same file as 2a-2c.

### 2e. `BedSound.catchUp` warns on every read (Cleanup — log spam)

**Location**: `BedSound.java:247-255` (`catchUp`); compare the render path's
once-flag at `:230` and `:293-296`.

**Root cause**: `catch (RuntimeException e) { FoleyClient.LOGGER.warn("Foley bed
rejected its scene", e); }` runs on every stream read (~2×/s) while a scene
stays invalid; the render path has the `warned` guard, this path has none.

**Change**: add `private boolean sceneWarned;` beside `warned` (line 230) and
gate the warning: `if (!sceneWarned) { sceneWarned = true; FoleyClient.LOGGER.warn(…); }`.

**Dependencies**: none (different file from 2a-2d).

### 2f. Cicada chorus can start before the previous ends (Minor)

**Location**: `Life.java:248-249`.

**Root cause**: `nextCicadas = ticks + 110 + random.nextInt(40)` = 5.5–7.5 s,
but the `cicadas` one-shot is 6.0 s long
(`crates/foley-synth/src/outdoors.rs:216`), so consecutive choruses can overlap
by up to 0.5 s (1 in 8 chance of the same variant).

**Change**: `nextCicadas = ticks + 120 + random.nextInt(40);` — the comment
above it already says "one chorus after another".

**Dependencies**: none.

### Task 2 verification

```
cd mod
JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests '*SurfControllerTest*' --tests '*WindControllerTest*' --no-configuration-cache
```

then the full Java suite:

```
JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --no-configuration-cache
```

Expected: `BUILD SUCCESSFUL`, no test failures. Report the exact command and the
final summary line.

```
[acceptance]
criteria: ./gradlew test passes from mod/ with no failures, and the implementer report quotes the exact commands and the BUILD SUCCESSFUL lines
deliverable: mod/src/main/java/io/github/cumsauce/foley/audio/{SurfController,WindScene,BedSound,Life}.java, mod/src/test/java/io/github/cumsauce/foley/audio/SurfControllerTest.java
[/acceptance]
```

---

## Task 3: Docs — stale counts in `docs/PLAN.md`

**Location**: `docs/PLAN.md:158` ("Each of 12 species") and `:164`
("12 species × 4 songs").

**Root cause**: the M5 results text predates M13. The catalogue is 57 species
(`crates/foley-ffi/src/lib.rs:58, 113-114`; species index 0..=56; 57 `bird_*`
entries in `mod/src/main/resources/assets/foley/sounds.json`).

**Change**: correct both to the M13 figure, e.g. "Each of 57 species (grown
from 9 in M5 to 57 in M13)" and "57 species × 4 songs". Do not restructure the
surrounding prose.

**Dependencies**: none.

```
[acceptance]
criteria: docs/PLAN.md no longer states 12 bird species in the M5 results sections; the numbers agree with lib.rs (57) and sounds.json
deliverable: docs/PLAN.md
[/acceptance]
```

---

## Non-goals (deliberately not in this plan)

- The client gametest (`./gradlew runClientGameTest`) remains gated to
  `workflow_dispatch`; wiring it to push is a separate CI change with its own
  risk, and M15/M16 (Nether/End) are untouched.
- No golden-sample or spectral audio assertions: they need a committed corpus
  and a numeric tolerances policy, which is a design task, not a cheap fix.

## Quick wins first

Task 3 is a two-line doc change. Task 1 is the highest-value Rust batch
(1a is the only audibly user-visible bug). Task 2 is the Java side and is
file-disjoint from both, so all three can run in parallel.
