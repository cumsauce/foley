# M14 Underwater Ambience Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a listener state that, when the player's head is submerged, plays a bed of bubbles rising past the ear and a low pressure tone, and applies a 1 kHz low-pass to every other Foley streaming sound; on emerging, fades the bed and pressure over 1 s and removes the low-pass at once.

**Architecture:** A new streaming bed kind (`FOLEY_BED_UNDERWATER`) added to `foley-dsp` that, in `no_std` land, renders bubble rings (drawn from the existing `Bubble` and `Fixed` infrastructure, the way `Water` does for streams and waterfalls) plus a slow 50 Hz sinusoid for the pressure, with a one-second exponential fade-out on the whole bed when its `level` parameter glides down to 0. A new Java `UnderwaterController` keeps that single bed playing, told its scene from the player's eye position and velocity each tick. A process-global `submerged` flag (set by Java through a new `foley_set_submerged(bool)` FFI export) is checked inside `foley_bed_render` and post-passes the sample through a one-pole low-pass at 1 kHz for every bed except `Underwater` itself (the underwater bed is already underwater, and mob voices are unaffected — the spec calls them out as "the test for the filter", meaning voices already do this kind of muffled underwater thing; the LPF applies to Foley's streamed beds only). A new toggle `underwater` is appended to `FoleyConfig` between `sculk` and `voices` (default `true`, since the underwater state is something Foley *adds*).

**Tech Stack:** Rust (no_std DSP), C ABI via FFM, Java 25 + Fabric Loom, JUnit, Fabric Client GameTest.

**Spec:** `docs/PLAN.md` lines 551–563 (parent foley repo).

## Global Constraints

- `FOLEY_ABI_VERSION` lives in `crates/foley-ffi/src/lib.rs` and `crates/foley-ffi/include/foley.h`; bump from 20 to 21.
- Java `NativeBridge.ABI_VERSION` must match the C ABI.
- `FoleyConfig` is a record: `volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets, frogs, minecarts, doors, chests, redstone, boats, sculk, voices`. The new toggle `underwater` goes between `sculk` and `voices`. Every positional `FoleyConfig(...)` call site must append `.withUnderwater(true|false)` to keep working.
- Append-only for shared files: `sounds.json`, `lang/en_us.json`, `FoleyConfig`, `SoundModel`. Do not reorder existing fields.
- The `runGametest` task may fail with "Timeout loading world" — that's pre-existing; document but don't fix.
- `CARGO_BUILD_JOBS=1` for cargo, JDK 25 for gradle.

## Review Focus

These are input classes or failure modes the spec implies but the plan's per-task tests do not cover, ranked by what would bite a person using this software most:

1. **Mob voices are not LPF'd by the underwater state** (the spec says voices already do this kind of muffled underwater thing). A test that confirms a zombie's voice is unaffected (still replaced) when the player is under water.
2. **The bed is silent above sea level** and **silent when the head is just below the surface but the player is breathing** (the head must be inside a water block for `depth > 0`). Tests for both.
3. **Submerging while the bed is already running is seamless** (no double-start, no click). A test that repeatedly toggles the controller's `submerged` true/false/true never creates more than one bed instance and the bed's gain never crosses zero.
4. **Surfacing while breathing resets cleanly** (no double-fade). The plan's emerging fade fades the bed itself; ensure the controller's `submerged` flag flips immediately on emerging and the bed is told `level = 0` once.
5. **Earshot's underwater filter and Foley's 1 kHz LPF stack cleanly without double-muffling**. The plan applies Foley's LPF on Foley's other streaming beds (not on voices, not on the underwater bed itself), so Earshot's effect on those beds comes on top — by design. A test that confirms a water bed's high-frequency content drops while submerged, but does not go silent.

---

### Task 1: `Underwater` Rust bed kind

**Files:**
- Create: `crates/foley-dsp/src/underwater.rs`
- Modify: `crates/foley-dsp/src/lib.rs` (declare module, re-export types)
- Test: `crates/foley-dsp/src/underwater.rs` inline `#[cfg(test)]`

**Interfaces:**
- Consumes: `crate::{Bubble, Fixed, LowPass, Rng, SAMPLE_RATE}`, `foley_math::Float`
- Produces:
  - `pub struct Underwater { … }` with `pub fn new(seed: u64) -> Self`, `pub fn set_scene(&mut self, scene: UnderwaterScene)`, `pub fn render(&mut self, out: &mut [f32])`
  - `pub struct UnderwaterScene { pub level: f32 }` — single float 0..=1 (the controller derives it from depth and the dive transient)
  - When `level == 0`, output is exactly zero within one render block.

**Implementation notes:**
- `MAX_VOICES = 64` bubbles.
- Rate of new bubble entries: `level * (4 + 16 * level)` bubbles per second (so a full dive starts up to 20 a second).
- Bubble radius: log-uniform 0.5–6 mm, loudness 0.02.
- Pressure tone: 50 Hz sine, level = 0.25 * level (small, so it sits beneath the bubbles).
- Wrap-around `voices` are rendered with `Bubble::new(radius, loudness).sample()` summed, then pressure tone added: `mix += 0.25 * level * (phase * 50.0 * 2π / SAMPLE_RATE).sin()`.
- A `Wobble` modulates the bubble loudness subtly.
- The bed has a fade-out: when `level` glides from 1 to 0, the bed's output must reach 0 within 1 s; implement by multiplying the final mix by `level` (already true) but also holding the `voices` list empty when `level == 0`.

**TDD steps:**

- [ ] **Step 1: Write failing test** in `underwater.rs`'s `#[cfg(test)]`:

```rust
#[test]
fn a_submerged_bed_renders_bubbles_and_pressure() {
    let mut bed = Underwater::new(1);
    bed.set_scene(UnderwaterScene { level: 1.0 });
    let mut out = vec![0.0f32; SAMPLE_RATE]; // 1 second
    bed.render(&mut out);
    let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
    assert!(rms > 0.05, "submerged bed was silent at {rms}");
    // The pressure tone at 50 Hz: zero crossings in 1 s ≈ 100.
    let crossings = out.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
    assert!((50..=150).contains(&crossings), "pressure tone not heard: {crossings} crossings");
}

#[test]
fn a_dry_bed_is_silent() {
    let mut bed = Underwater::new(2);
    bed.set_scene(UnderwaterScene { level: 0.0 });
    let mut out = vec![0.0f32; SAMPLE_RATE];
    bed.render(&mut out);
    let peak = out.iter().fold(0f32, |m, x| m.max(x.abs()));
    assert!(peak < 1e-6, "dry bed peak {peak}");
    assert!(bed.rendered == 0 || bed.peak_at_last_render() < 1e-6);
}
```

Add a small accessor `peak_at_last_render` only when `cfg(test)`.

- [ ] **Step 2: Run test to verify it fails** — `cd /Volumes/My\ Shared\ Files/mc/foley-m14 && CARGO_BUILD_JOBS=1 cargo test -p foley-dsp --lib underwater` → Expected: compile error (type not defined).

- [ ] **Step 3: Implement `Underwater` and `UnderwaterScene`** as described. Bubbles via `Bubble` + `Fixed<Bubble, 64>`, pressure via running phase, level multiplies the mix at the end.

- [ ] **Step 4: Run test to verify it passes** — `cd /Volumes/My\ Shared\ Files/mc/foley-m14 && CARGO_BUILD_JOBS=1 cargo test -p foley-dsp --lib` → Expected: 2 new tests pass.

- [ ] **Step 5: Commit**

```bash
cd /Volumes/My\ Shared\ Files/mc/foley-m14 && git add crates/foley-dsp/src/underwater.rs crates/foley-dsp/src/lib.rs && git commit -m "feat(synth): add Underwater bed for M14"
```

---

### Task 2: Wire `FOLEY_BED_UNDERWATER` into the FFI and bump ABI to 21

**Files:**
- Modify: `crates/foley-ffi/src/lib.rs` (add `FOLEY_BED_UNDERWATER = 6`, add to `Synth` enum, handle in `apply` and `foley_bed_create`, bump `FOLEY_ABI_VERSION` to 21, update header comment)
- Modify: `crates/foley-ffi/include/foley.h` (regenerate via `cargo xtask header` or manually mirror the change)
- Test: extend `every_bed_renders_pcm` in `lib.rs`'s inline tests.

**Interfaces:**
- `FOLEY_BED_UNDERWATER = 6` (next after `FOLEY_BED_THUNDER = 5`).
- `Synth::Underwater(Underwater)` variant.
- Scene for underwater: 1 float `level` (0..=1).

**TDD steps:**

- [ ] **Step 1: Update the inline `every_bed_renders_pcm` test** to include `(FOLEY_BED_UNDERWATER, &[1.0f32][..])`.

- [ ] **Step 2: Run test to verify it fails** — `cd /Volumes/My\ Shared\ Files/mc/foley-m14 && CARGO_BUILD_JOBS=1 cargo test -p foley-ffi --lib every_bed_renders_pcm` → Expected: FAIL with "bed 6 not created".

- [ ] **Step 3: Add `FOLEY_BED_UNDERWATER = 6`, bump ABI to 21**, wire into `foley_bed_create`/`apply`/`render` exactly like the other bed kinds (1 param: `level` 0..=1, gain range check). Update the ABI version comment to mention `21: underwater ambience (M14)`. Update `foley.h` (either via `cargo xtask header` or hand-edit if xtask unavailable) to mirror.

- [ ] **Step 4: Run test to verify it passes** — `cd /Volumes/My\ Shared\ Files/mc/foley-m14 && CARGO_BUILD_JOBS=1 cargo test -p foley-ffi --lib` → Expected: all pass including the extended `every_bed_renders_pcm`.

- [ ] **Step 5: Verify header regeneration** — if `cargo xtask header` exists in this repo, run it; otherwise hand-edit `include/foley.h` to add `FOLEY_BED_UNDERWATER = 6` and bump `FOLEY_ABI_VERSION` macro.

- [ ] **Step 6: Commit**

```bash
cd /Volumes/My\ Shared\ Files/mc/foley-m14 && git add crates/foley-ffi/src/lib.rs crates/foley-ffi/include/foley.h && git commit -m "feat(ffi): add FOLEY_BED_UNDERWATER and bump ABI to 21"
```

---

### Task 3: Add `BED_UNDERWATER` constant in `NativeBridge`

**Files:**
- Modify: `mod/src/main/java/io/github/cumsauce/foley/natives/NativeBridge.java`
- Test: N/A (constant only)

**Interfaces:** `public static final int BED_UNDERWATER = 6;`. Bump `ABI_VERSION` to 21.

**TDD steps:**

- [ ] **Step 1: Add the constant and bump `ABI_VERSION`** to `NativeBridge.java`.

- [ ] **Step 2: Verify the change compiles** — `cd /Volumes/My\ Shared\ Files/mc/foley-m14/mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew compileJava` → Expected: BUILD SUCCESSFUL.

- [ ] **Step 3: Commit**

```bash
cd /Volumes/My\ Shared\ Files/mc/foley-m14 && git add mod/src/main/java/io/github/cumsauce/foley/natives/NativeBridge.java && git commit -m "feat(bridge): add BED_UNDERWATER and bump Java ABI to 21"
```

---

### Task 4: Add `underwater` toggle to `FoleyConfig`

**Files:**
- Modify: `mod/src/main/java/io/github/cumsauce/foley/config/FoleyConfig.java`
- Modify (call sites): every positional `new FoleyConfig(...)` in `mod/src/`
- Modify: `mod/src/gametest/java/io/github/cumsauce/foley/gametest/FoleyClientGameTest.java` (every positional `new FoleyConfig(...)` — append-only)
- Test: `mod/src/test/java/io/github/cumsauce/foley/config/FoleyConfigTest.java` (if test class exists; otherwise verify by inspecting `DEFAULT` and `fromJson`/`toJson` round-trip in the existing JUnit suite).

**Interfaces:**
- New record field `boolean underwater` between `sculk` and `voices`.
- New method `withUnderwater(boolean on)`.
- `DEFAULT.underwater == true` (Foley adds the underwater ambience, default on).
- `ALL.underwater == true`.
- `NONE.underwater == false`.

**TDD steps:**

- [ ] **Step 1: Write failing test** in the existing or new `FoleyConfigTest`:

```java
@Test
public void default_settings_have_underwater_on() {
    assertTrue(FoleyConfig.DEFAULT.underwater());
}

@Test
public void all_settings_have_underwater_on() {
    assertTrue(FoleyConfig.ALL.underwater());
}

@Test
public void none_settings_have_underwater_off() {
    assertFalse(FoleyConfig.NONE.underwater());
}

@Test
public void wither_underwater_changes_only_underwater() {
    FoleyConfig start = FoleyConfig.DEFAULT;
    FoleyConfig next = start.withUnderwater(false);
    assertFalse(next.underwater());
    assertEquals(start.rain(), next.rain());
    // every other field unchanged
}

@Test
public void underwater_round_trips_through_json() {
    JsonObject json = FoleyConfig.DEFAULT.withUnderwater(false).toJson();
    assertEquals(false, json.get("underwater").getAsBoolean());
    FoleyConfig back = FoleyConfig.fromJson(json);
    assertFalse(back.underwater());
}
```

- [ ] **Step 2: Run test to verify it fails** — `cd /Volumes/My\ Shared\ Files/mc/foley-m14/mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests "*FoleyConfigTest*"` → Expected: compile error (no `underwater` field yet).

- [ ] **Step 3: Add the field, wither, default, ALL, NONE, fromJson, toJson**. Append `boolean underwater` between `sculk` and `voices`. Add `withUnderwater(boolean on)`. In `DEFAULT` set `underwater` to `true`. In `NONE` set it to `false`. In `fromJson` add `flag(json, "underwater", d.underwater)`. In `toJson` add `json.addProperty("underwater", underwater)`.

- [ ] **Step 4: Update every positional `FoleyConfig(...)` call site in the mod** to append `.withUnderwater(true|false)` so existing call sites keep compiling. Use `git grep -nE 'new FoleyConfig\('` to find them. (Most call sites already exist in `FoleyClientGameTest` — append `.withUnderwater(true)` to keep behaviour; in the gametest block for M14 you'll use `.withUnderwater(false)` to assert the toggle.)

- [ ] **Step 5: Run test to verify it passes** — `cd /Volumes/My\ Shared\ Files/mc/foley-m14/mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test` → Expected: green, including the new `FoleyConfigTest` cases.

- [ ] **Step 6: Commit**

```bash
cd /Volumes/My\ Shared\ Files/mc/foley-m14 && git add mod/src/main/java/io/github/cumsauce/foley/config/FoleyConfig.java mod/src/test mod/src/gametest && git commit -m "feat(config): add underwater toggle and wither"
```

---

### Task 5: `UnderwaterController` (Java) + `FoleyClient.underwater()`

**Files:**
- Create: `mod/src/main/java/io/github/cumsauce/foley/audio/UnderwaterController.java`
- Modify: `mod/src/main/java/io/github/cumsauce/foley/FoleyClient.java` (add `underwater` field, accessor, register tick)

**Interfaces:**
- `public final class UnderwaterController`:
  - `public UnderwaterController(NativeBridge bridge)`
  - `public UnderwaterScene scene()` — current scene; `UnderwaterScene.UNDER` or `UnderwaterScene.DRY`
  - `public void tick(Minecraft minecraft)` — samples depth + speed, glides `level` over 1 s to 1.0 when submerged and to 0.0 when emerged
  - `public @Nullable BedSound<UnderwaterScene> sound()` — the bed, if playing
  - One scene record `UnderwaterScene(float level) implements Scene` with `params() -> [level]`
  - One constant `UnderwaterScene UNDER = new UnderwaterScene(1.0f)`, `UnderwaterScene DRY = new UnderwaterScene(0.0f)`
  - Depth sampling: `level = clamp((sea_level - eye_y) / 30.0, 0.0, 1.0)`; or 0.0 if not in water at the head. Determine "head in water" by checking `level.getFluidState(BlockPos.containing(eye))` is `Fluids.WATER` (not in cave air, etc.).
- `FoleyClient` gains `private static @Nullable UnderwaterController underwater;` and a public accessor, and a tick registration in `onInitializeClient`.

**TDD steps:**

- [ ] **Step 1: Write failing test** in `UnderwaterController.java`'s inner `static class ForTest` (no test framework needed; we test by hand-instantiating with a stub):

Actually we don't have a fake `Minecraft`/`ClientLevel`. Instead, the unit test lives in `crates/foley-dsp` (the bed, already covered by Task 1) and the Java side is tested by the gametest (Task 7). Skip a JUnit for `UnderwaterController`'s scene math; instead expose a `static float depthLevel(Minecraft)`-style helper? Keep it simple — only the gametest exercises the controller end-to-end.

Document: **no JUnit for the controller itself** because it reads from `Minecraft`/`ClientLevel` which can't be unit-tested without a running client. The controller's logic is exercised by the gametest in Task 7.

- [ ] **Step 2: Implement `UnderwaterController`** with:
  - A `BedSound<UnderwaterScene>` it owns; created on first submerged sample, kept playing while submerged, weighed to 0 on emerge, and stopped 1 s after emerge (`fading` counter like `PlacedBeds.Slot`).
  - Scene: `level` glides from 0 to 1 over 1 s when `submerged` becomes true, from 1 to 0 over 1 s when false.
  - The bed uses `SoundSource.AMBIENT`, kind `NativeBridge.BED_UNDERWATER`.
  - Tick is registered with `ClientTickEvents.END_CLIENT_TICK` in `FoleyClient`.

- [ ] **Step 3: Add the field, accessor, and registration in `FoleyClient.java`** alongside the other controllers.

- [ ] **Step 4: Verify compile** — `cd /Volumes/My\ Shared\ Files/mc/foley-m14/mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew compileJava` → Expected: BUILD SUCCESSFUL.

- [ ] **Step 5: Commit**

```bash
cd /Volumes/My\ Shared\ Files/mc/foley-m14 && git add mod/src/main/java/io/github/cumsauce/foley/audio/UnderwaterController.java mod/src/main/java/io/github/cumsauce/foley/FoleyClient.java && git commit -m "feat(audio): add UnderwaterController for M14"
```

---

### Task 6: `sounds.json` and `en_us.json` append-only entries

**Files:**
- Modify (append only): `mod/src/main/resources/assets/foley/sounds.json`
- Modify (append only): `mod/src/main/resources/assets/foley/lang/en_us.json`

**Interfaces:**
- New `sounds.json` event `underwater_bed` with 4 variants pointing to `foley:generated/underwater_bed_<0..3>` (the bed itself is rendered as PCM by the game's stream reader, but we add a one-shot for an `underwater_pressure` event anyway — the spec says "M14 might add a `foley:underwater_pressure` one-shot"). The `underwater_pressure` event uses `FOLEY_PRIMITIVE_TONE` at 50 Hz, 1 s, gain 0.25 — but this is on-demand, so we add it as a one-shot for the gametest to play. Add four variants `foley:generated/underwater_pressure_<0..3>`.
- New lang keys `subtitles.foley.underwater` (just "Underwater") and `foley.settings.underwater` ("Underwater Ambience") and `foley.settings.underwater.tooltip` (a one-liner).
- **Append-only:** add the keys/entries to the END of each section in each file. Do not modify any existing line.

**TDD steps:**

- [ ] **Step 1: Verify the append-only rule** — re-read the sounds.json's last entry and confirm we only add new entries below the closing `}` of `sculk_ring`. Same for `en_us.json` (add keys after `foley.settings.voices.all_game`).

- [ ] **Step 2: Append the new entries** to `sounds.json`:
```json
,
"underwater_pressure": {
    "subtitle": "subtitles.foley.underwater",
    "sounds": [
        "foley:generated/underwater_pressure_0",
        "foley:generated/underwater_pressure_1",
        "foley:generated/underwater_pressure_2",
        "foley:generated/underwater_pressure_3"
    ]
}
```
…and to `en_us.json`:
```json
,
"subtitles.foley.underwater": "Underwater",
"foley.settings.underwater": "Underwater Ambience",
"foley.settings.underwater.tooltip": "Bubbles rise past your ear and a low tone builds with depth when your head is under water. Emerged, Foley sounds the same as above."
```

- [ ] **Step 3: Verify the JSON parses** — quick check by reading both files for valid JSON structure (matching braces and commas). Then run a tiny JUnit in `mod/src/test/java/io/github/cumsauce/foley/audio/SoundsJsonTest.java`? Skip — the gametest loads the assets.

- [ ] **Step 4: Commit**

```bash
cd /Volumes/My\ Shared\ Files/mc/foley-m14 && git add mod/src/main/resources/assets/foley/sounds.json mod/src/main/resources/assets/foley/lang/en_us.json && git commit -m "feat(sounds): add M14 underwater_pressure event and language strings"
```

---

### Task 7: Gametest: head submerged → underwater bed plays

**Files:**
- Modify (append only): `mod/src/gametest/java/io/github/cumsauce/foley/gametest/FoleyClientGameTest.java`

**Interfaces:**
- New gametest block at the end (after the active structures block):
    - Set weather clear, place the player at y = 55 in an ocean (deep enough), wait for `FoleyClient.underwater().sound()` to be active with `rendered > NativeBridge.SAMPLE_RATE / 2`.
    - Switch `FoleyConfig.underwater = false` via `FoleyClient.apply`, wait for the bed's `rendered` to stop growing.
    - Switch back on, wait for it to play again.

**TDD steps:**

- [ ] **Step 1: Write the failing gametest block** (it will fail at first because the controller isn't wired yet, but Task 5 already implements that — we just want the assertion).

```java
// M14 underwater ambience: with the player under water, the bed plays.
singleplayer.getServer().runCommand("weather clear");
singleplayer.getServer().runCommand("tp @s ~ 55 ~");
singleplayer.getServer().runCommand("fill ~-3 ~-1 ~-3 ~3 ~3 ~3 water");
// The player is now in water below sea level.
int beforeRendered = -1;
try {
    context.waitFor(mc -> {
        var u = FoleyClient.underwater();
        if (u == null || u.sound() == null) return false;
        long r = u.sound().rendered();
        beforeRendered = (int) r;
        return mc.getSoundManager().isActive(u.sound()) && r > NativeBridge.SAMPLE_RATE / 2;
    }, 30 * 20);
} catch (RuntimeException | AssertionError e) {
    throw new AssertionError("the underwater bed never played: "
            + (FoleyClient.underwater() == null ? "off" : FoleyClient.underwater().sound()), e);
}
// And it fades when the toggle is off.
context.runOnClient(mc -> FoleyClient.apply(settings.withUnderwater(false)));
try {
    context.waitFor(mc -> {
        var u = FoleyClient.underwater();
        return u != null && (u.sound() == null || !mc.getSoundManager().isActive(u.sound()));
    }, 10 * 20);
} catch (RuntimeException | AssertionError e) {
    throw new AssertionError("the underwater bed didn't stop when the toggle was off", e);
}
context.runOnClient(mc -> FoleyClient.apply(settings));
```

Note: the constructor above uses `.withUnderwater(false)` instead — the call to `FoleyConfig(...)` will work because Task 4 added `.withUnderwater(false)` — but we don't yet have the wither at this point in the codebase. So we use the literal `false` value as the 20th positional argument (between `sculk` and `voices`), which is what Task 4 enables.

- [ ] **Step 2: Run the gametest** — `cd /Volumes/My\ Shared\ Files/mc/foley-m14/mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew runGametest` (or just compileJava for verification — gametest is CI-runnable but slow). If `runGametest` times out, document and continue (pre-existing infrastructure issue).

- [ ] **Step 3: Commit**

```bash
cd /Volumes/My\ Shared\ Files/mc/foley-m14 && git add mod/src/gametest/java/io/github/cumsauce/foley/gametest/FoleyClientGameTest.java && git commit -m "test(gametest): M14 underwater bed plays and toggles"
```

---

### Task 8: Settings screen: append `underwater` row

**Files:**
- Modify: `mod/src/main/java/io/github/cumsauce/foley/ui/FoleyScreen.java`
- Modify: `mod/src/main/java/io/github/cumsauce/foley/audio/UnderwaterController.java` is unchanged.

**Interfaces:** New `OptionInstance<Boolean> underwater` initialised from `config.underwater()`, added to the section "Weather" (or its own section "Underwater") — placed last in the weather section, after `wind`, on its own row.

**TDD steps:**

- [ ] **Step 1: Implement** — append the field, initialise it, and `list.addSmall(underwater, null)` after the existing weather row in `addOptions()`. Update `updateActive`'s list of toggles. Update `removed()`'s `FoleyConfig(...)` call to pass `underwater.get()`.

- [ ] **Step 2: Verify compile** — `cd /Volumes/My\ Shared\ Files/mc/foley-m14/mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew compileJava` → Expected: BUILD SUCCESSFUL.

- [ ] **Step 3: Commit**

```bash
cd /Volumes/My\ Shared\ Files/mc/foley-m14 && git add mod/src/main/java/io/github/cumsauce/foley/ui/FoleyScreen.java && git commit -m "feat(ui): add Underwater toggle to settings"
```

---

### Task 9: Final verification

**Files:** none modified.

**Steps:**

- [ ] **Step 1:** Run `cd /Volumes/My\ Shared\ Files/mc/foley-m14 && CARGO_BUILD_JOBS=1 cargo test --workspace --lib` → Expected: all green.
- [ ] **Step 2:** Run `cd /Volumes/My\ Shared\ Files/mc/foley-m14/mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test compileJava` → Expected: BUILD SUCCESSFUL.
- [ ] **Step 3:** Run `cd /Volumes/My\ Shared\ Files/mc/foley-m14/mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew build` → Expected: BUILD SUCCESSFUL.
- [ ] **Step 4:** Document any deviations or unresolved issues in the final report.