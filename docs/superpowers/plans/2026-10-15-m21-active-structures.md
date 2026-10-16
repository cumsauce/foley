# M21 — Active Structures Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Foley makes the world's active structures audible: minecarts clatter on rails and rush past, doors creak open and shut, chests and shulker boxes open with a soft puff, redstone clicks on state changes, boats lap water at the hull, and sculk sensors ring the sculk around them. The six sub-systems are joined under one `ActiveStructures` class on the client tick and one extension to `Replacements.replace` for the entity/block-entity sounds that aren't block-sound events.

**Architecture:** Reuse M4's `FOLEY_CONTACT_*` for creaks and clicks (the modal synthesis already models wood/metal hinges and stone-on-stone clicks); reuse the `bubble` primitive (M0/F2) for the boat's hull lapping; add one new `FOLEY_SHOT_SCULK_RING` for the sculk-sensor triggered tone. On the Java side, `ActiveStructures` ticks once every 10 ticks, scans for nearby rails/boats/sculk sensors and their target blocks, and plays one-shots (minecart wheel impacts, redstone state changes) or starts/stops a small per-entity bed (boat lapping). `Replacements.replace` learns the entity events to skip (door open/close, chest open, shulker open, minecart rolling, boat ambient).

**Tech Stack:** Rust (foley-dsp, foley-synth, foley-ffi), Java 21 (mod), Fabric 1.21.x, cbindgen (`cargo xtask header`).

**Spec:** `docs/PLAN.md` lines 658–682 (M21 plan).

## Global Constraints

- Bump `FOLEY_ABI_VERSION` to **20** in `crates/foley-ffi/src/lib.rs` and `crates/foley-ffi/include/foley.h`, and the Java `NativeBridge.ABI_VERSION` to 20. Java refuses a library whose version differs.
- Add one new one-shot kind `FOLEY_SHOT_SCULK_RING = 16` (a triggered low-Q tone that rings like a struck bowl — see `foley-dsp`'s `Bubble` ring for the model). Its `size` argument is the bowl's Q (0.5 to 20, default 6) and the seed is mixed with the sensor's `BlockPos` hash so each sensor has a stable ring.
- All other M21 sounds reuse the existing FFI: `FOLEY_CONTACT_*` (M4) for creaks and clicks, `FOLEY_PRIMITIVE_BUBBLE` (F2) for the boat's hull lapping, no new bed kind. The 8-stream budget is unaffected (M21 doesn't add a bed).
- The `ActiveStructures` tick is the only new client-tick entry; it runs at 10-tick cadence (`SAMPLE_EVERY = 10`) following the `Surroundings` and `WindController` patterns.
- `Replacements.replace` gets a new arm: a `Set<Identifier>` of M21 entity/block-entity events to skip when Foley layers under them, parallel to the existing `RAIN`, `WATER`, `LAVA`, `FIRE`, `DRIP_*` sets. Each set has a per-toggle gate in `FoleyConfig` (see Task 5).
- M21 adds **five** new `FoleyConfig` boolean toggles, all default **on** (per the "every sound Foley replaces can be given back to the game" rule in the existing config): `minecarts`, `doors`, `chests`, `redstone`, `boats`, `sculk`. The booleans sit after `frogs` and before the long `voices` field. The `Sculk` toggle gates both the sensor ring and the ring of nearby sculk blocks; the boat toggle gates the per-boat bed; the minecart toggle gates wheel impacts and rolling rustle; doors, chests, redstone each gate their own.
- Boat lapping as a per-boat bed could exceed the 8-stream budget if many boats are nearby; cap at `FoleyBeds.MOST` per loaded area (the same cap `AddonBeds` enforces, 2 by default in `FoleyBeds.MOST`).
- Sculk ring is a one-shot per sensor trigger, not a bed; the trigger is detected by scanning the world's `blockEntity` list (or by listening to `ClientGamePacketListener` for a `LevelEvent`-style packet — see the existing `ThunderController` pattern).
- All new Java code follows the file naming and package conventions of the existing `audio` package.

## File Structure

| Path | Action | Responsibility |
| --- | --- | --- |
| `crates/foley-synth/src/sculk.rs` | create | `pub fn sculk_ring(q: f32, seed: u64) -> Vec<f32>` — a triggered low-Q tone (sculk-sensor ring) |
| `crates/foley-synth/src/lib.rs` | modify | re-export `sculk_ring` |
| `crates/foley-ffi/src/lib.rs` | modify | `FOLEY_SHOT_SCULK_RING = 16`, accept in `foley_render_one_shot`, ABI bump to 20, extend the `every_bed_renders_pcm` / one-shot tests |
| `crates/foley-ffi/include/foley.h` | regenerate | via `cargo xtask header` |
| `mod/src/main/java/io/github/cumsauce/foley/natives/NativeBridge.java` | modify | `SHOT_SCULK_RING = 16`, `ABI_VERSION = 20` |
| `mod/src/main/java/io/github/cumsauce/foley/config/FoleyConfig.java` | modify | six new boolean fields and toggles, withers, JSON I/O, default-on; update `FoleyConfigTest` to round-trip |
| `mod/src/main/java/io/github/cumsauce/foley/audio/ActiveStructures.java` | create | the per-10-tick scan + boat lapping bed manager; mirrors the `WindController` style but with one-shots instead of streaming beds (except for the boat and the minecart rush) |
| `mod/src/main/java/io/github/cumsauce/foley/audio/BoatBed.java` | create | small `SurroundBeds<BoatScene>`-style manager for one boat, capped at `FoleyBeds.MOST` total boats |
| `mod/src/main/java/io/github/cumsauce/foley/audio/BoatScene.java` | create | record with `params() : float[] { ... }` for the bubble primitive per tick |
| `mod/src/main/java/io/github/cumsauce/foley/audio/MinecartBed.java` | create | small `SurroundBeds<MinecartScene>`-style manager for one minecart's rush; capped at 2 active beds total |
| `mod/src/main/java/io/github/cumsauce/foley/audio/MinecartScene.java` | create | record with `params() : float[] { speed }` — gain is proportional to speed |
| `mod/src/main/java/io/github/cumsauce/foley/audio/Replacements.java` | modify | add a `M21` set of events to skip and a helper that decides per-toggle |
| `mod/src/main/java/io/github/cumsauce/foley/api/SoundModel.java` | modify | add `record SculkRing(float q)` variant (a new `SoundModel`) |
| `mod/src/main/java/io/github/cumsauce/foley/api/SoundCodecs.java` | modify | add the codec entry for `SculkRing` |
| `mod/src/main/java/io/github/cumsauce/foley/audio/AddonSounds.java` | modify | dispatch `SculkRing` to `renderOneShot(SHOT_SCULK_RING, q, seed)` |
| `mod/src/main/java/io/github/cumsauce/foley/audio/Sounds.java` | (no change) | events are added through the `Sounds.event(name)` call from `Replacements`/`ActiveStructures` |
| `mod/src/main/resources/assets/foley/sounds.json` | modify | add `door_wood`, `door_iron`, `chest_open`, `shulker_open`, `redstone_click`, `minecart_wheel`, `boat_lap`, `sculk_ring` events; 4 variants each; the door/chest/click/wheel are `tap_<material>`-style layered one-shots; the boat lap and sculk ring are `generated/<name>_<n>` style generated one-shots |
| `mod/src/main/java/io/github/cumsauce/foley/FoleyClient.java` | modify | instantiate `ActiveStructures`, register its `tick`, expose accessor `activeStructures()` |
| `mod/src/gametest/java/io/github/cumsauce/foley/gametest/FoleyClientGameTest.java` | modify | update `FoleyConfig` construction with the six new booleans; add an M21 test step that places rails, a chest, a door, a boat and a sculk sensor and asserts each M21 event fires |

---

## Task 1: Sculk-ring synth in foley-synth (TDD)

**Files:**
- Create: `crates/foley-synth/src/sculk.rs`
- Modify: `crates/foley-synth/src/lib.rs` (re-export)
- Test: inline `#[cfg(test)]` in `sculk.rs`

**Interfaces:**

```rust
/// A sculk sensor's triggered tone: a low-Q bowl ring at ~200 Hz,
/// slowly decaying (the sculk "vibrates" with the trigger's intensity).
pub fn sculk_ring(q: f32, seed: u64) -> Vec<f32>;
```

The ring is `Bubble`-like: a damped sinusoid at ~200 Hz with a slow amplitude decay (τ ≈ 1.5 s). `q` controls the damping (high Q = ringy, low Q = thud). The `seed` is mixed into the phase so each call has a stable starting phase.

- [ ] **Step 1: Write failing tests in `crates/foley-synth/src/sculk.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::SAMPLE_RATE;
    fn rms(s: &[f32]) -> f32 { (s.iter().map(|x| x*x).sum::<f32>() / s.len().max(1) as f32).sqrt() }
    fn ring_at(out: &[f32], start: usize) -> f32 {
        // Zero-crossing pitch around the loudest 10 ms window.
        let win = &out[start..(start + 480).min(out.len())];
        let loudest = win.iter().enumerate().fold((0, 0f32), |m, (i, x)| if x.abs() > m.1 {(i, x.abs())} else {m}).0;
        let w = &win[loudest.saturating_sub(240)..(loudest+240).min(win.len())];
        w.windows(2).filter(|x| x[0] <= 0.0 && x[1] > 0.0).count() as f32 * (SAMPLE_RATE as f32 / w.len() as f32)
    }
    #[test]
    fn ring_is_a_low_pitched_damped_tone() {
        let out = sculk_ring(6.0, 1);
        let p = ring_at(&out, 0);
        assert!((150.0..=300.0).contains(&p), "ring pitch {p} Hz");
        assert!(out.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
        assert!(out.len() < 4 * SAMPLE_RATE as usize);
    }
    #[test]
    fn higher_q_rings_longer() {
        let low = sculk_ring(0.5, 2);
        let high = sculk_ring(20.0, 2);
        // RMS over the second half of the buffer.
        let half = |v: &[f32]| rms(&v[v.len()/2..]);
        assert!(half(&high) > 2.0 * half(&low), "high q {}/{} low q", half(&high), half(&low));
    }
    #[test]
    fn same_seed_is_deterministic() {
        assert_eq!(sculk_ring(6.0, 42), sculk_ring(6.0, 42));
    }
}
```

- [ ] **Step 2: Run; expect compile error on missing `sculk_ring`**

Run: `cargo test --package foley-synth sculk`
Expected: compile error: `cannot find function sculk_ring`.

- [ ] **Step 3: Implement `sculk_ring` in `crates/foley-synth/src/sculk.rs`**

A 200 Hz sine with an exponentially decaying amplitude (`exp(-t / τ)`, where `τ = 1.5 / q`); length 2.5 s; the `seed` XORed into the starting phase. No band-pass — the ring is a clean tone. The `q` parameter also drives a small inharmonic overtone at `f * 1.6` at 0.1 of the amplitude, so the bowl sounds like a bowl, not a perfect sine.

- [ ] **Step 4: Run tests; expect pass**

Run: `cargo test --package foley-synth sculk`
Expected: 3 tests pass.

- [ ] **Step 5: Re-export from `foley-synth/src/lib.rs`**

Add `pub mod sculk;` and `pub use sculk::sculk_ring;`.

- [ ] **Step 6: Commit**

```bash
git add crates/foley-synth/src/sculk.rs crates/foley-synth/src/lib.rs
git commit -m "feat(synth): add sculk_ring one-shot for M21"
```

---

## Task 2: `FOLEY_SHOT_SCULK_RING` in foley-ffi (TDD)

**Files:**
- Modify: `crates/foley-ffi/src/lib.rs`
- Modify: `crates/foley-ffi/include/foley.h` (regenerated)
- Test: `crates/foley-ffi/src/lib.rs` (existing one-shot tests cover new kinds)

- [ ] **Step 1: Bump `FOLEY_ABI_VERSION` to 20**

```rust
/// 20: active structures (M21): sculk sensor ring `FOLEY_SHOT_SCULK_RING`.
pub const FOLEY_ABI_VERSION: u32 = 20;
```

- [ ] **Step 2: Add `FOLEY_SHOT_SCULK_RING`**

```rust
/// A sculk sensor's triggered tone: `size` is the bowl's Q (0.5 to 20).
pub const FOLEY_SHOT_SCULK_RING: i32 = 16;
```

Update the doc comment for `FOLEY_SHOT_*` to include it.

- [ ] **Step 3: Add the dispatch in `foley_render_one_shot`**

```rust
FOLEY_SHOT_SCULK_RING if (0.5..=20.0).contains(&size) => sculk_ring(size, seed),
```

Add the corresponding arm to the trailing fallthrough `return Err(FfiError::invalid("size out of range"));` for `FOLEY_SHOT_SCULK_RING`.

- [ ] **Step 4: Regenerate the header**

Run: `cargo xtask header --check`
Expected: fails. Then `cargo xtask header`. Verify with re-run.

- [ ] **Step 5: Run ffi tests; expect pass**

Run: `cargo test --package foley-ffi`
Expected: all pass; the new `FOLEY_SHOT_SCULK_RING` is exercised by the existing one-shot test (the test iterates all one-shot kinds — confirm the test loop covers it; if it doesn't, add a `FOLEY_SHOT_SCULK_RING` case).

- [ ] **Step 6: Commit**

```bash
git add crates/foley-ffi/src/lib.rs crates/foley-ffi/include/foley.h
git commit -m "feat(ffi): add FOLEY_SHOT_SCULK_RING and bump ABI to 20"
```

---

## Task 3: Java `SoundModel.SculkRing` and AddonSounds dispatch

**Files:**
- Modify: `mod/src/main/java/io/github/cumsauce/foley/api/SoundModel.java`
- Modify: `mod/src/main/java/io/github/cumsauce/foley/api/SoundCodecs.java`
- Modify: `mod/src/main/java/io/github/cumsauce/foley/audio/AddonSounds.java`

- [ ] **Step 1: Add the `SculkRing` record to `SoundModel`**

Find the existing record variants in `SoundModel.java` (it's a sealed interface with records like `Bubble`, `Tone`, `Contact`, etc.). Add:

```java
record SculkRing(float q) implements SoundModel {}
```

- [ ] **Step 2: Add the codec entry in `SoundCodecs`**

Look at how `Bubble` (radius_mm) is encoded as JSON; add the analogous `sculk_ring` codec:

```java
case SoundModel.SculkRing m -> new com.google.gson.JsonObject();
```

The codec produces `{ "kind": "sculk_ring", "q": <m.q()> }`. The implementer follows the existing pattern; the JSON is consumed by `AddonSounds.parseEntry`.

- [ ] **Step 3: Add the dispatch in `AddonSounds.render`**

```java
case SoundModel.SculkRing m -> bridge.renderOneShot(NativeBridge.SHOT_SCULK_RING, m.q(), seed);
```

- [ ] **Step 4: Compile and run the existing addon tests**

Run: `cd mod && ./gradlew compileJava && ./gradlew test`
Expected: pass; no test broken (the SculkRing case is additive).

- [ ] **Step 5: Commit**

```bash
git add mod/src/main/java/io/github/cumsauce/foley/api/SoundModel.java mod/src/main/java/io/github/cumsauce/foley/api/SoundCodecs.java mod/src/main/java/io/github/cumsauce/foley/audio/AddonSounds.java
git commit -m "feat(api): add SculkRing SoundModel variant for M21"
```

---

## Task 4: Six new settings in `FoleyConfig`

**Files:**
- Modify: `mod/src/main/java/io/github/cumsauce/foley/config/FoleyConfig.java`
- Modify: `mod/src/test/java/io/github/cumsauce/foley/config/FoleyConfigTest.java`

The six new fields are `minecarts`, `doors`, `chests`, `redstone`, `boats`, `sculk`, all default `true`. They sit after `frogs` and before the long `voices` field. Each gets:
- A row in the record.
- A wither (`withMinecarts(boolean on)`, etc.).
- A `flag(json, "minecarts", d.minecarts)` line in `fromJson`.
- A `json.addProperty("minecarts", minecarts)` line in `toJson`.
- An updated `DEFAULT`, `ALL`, `NONE`.
- An updated test that round-trips a config with the new fields.

- [ ] **Step 1: Add the six fields to the record**

After `boolean frogs` and before `long voices`:

```java
public record FoleyConfig(int volume, boolean rain, boolean wind, boolean thunder, boolean water, boolean fire,
        boolean lava, boolean drips, boolean strikes, boolean footsteps, boolean birds, boolean crickets,
        boolean frogs, boolean minecarts, boolean doors, boolean chests, boolean redstone, boolean boats,
        boolean sculk, long voices) {
```

- [ ] **Step 2: Update `DEFAULT`, `ALL`, `NONE`**

```java
public static final FoleyConfig DEFAULT = new FoleyConfig(100, true, true, true, true, true, true, true, true, true,
        true, true, true, true, true, true, true, true, true, 0L);
public static final FoleyConfig ALL = DEFAULT;  // all on, including voices mask
public static final FoleyConfig NONE = new FoleyConfig(100, false, false, false, false, false, false, false, false,
        false, false, false, false, false, false, false, false, false, false, 0L);
```

- [ ] **Step 3: Add the withers**

```java
public FoleyConfig withMinecarts(boolean on) { return new FoleyConfig(volume, rain, on, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets, frogs, on /* minecarts */, doors, chests, redstone, boats, sculk, voices); }
// ... four more with the same pattern
```

(The `withMinecarts(boolean)` changes the minecarts toggle; the others follow.)

- [ ] **Step 4: Add JSON I/O**

In `fromJson`, add 6 `flag(json, "<name>", d.<name>)` calls. In `toJson`, add 6 `json.addProperty` calls. Reuse the same pattern as `birds`/`crickets`/`frogs`.

- [ ] **Step 5: Update the test**

Update `FoleyConfigTest.itRoundTrips` to pass 22 booleans + the voices long (the existing test already takes 14 booleans; it now needs 20 booleans). Update `missingOrBadValuesFallBackToTheirDefaults` to assert the new defaults.

- [ ] **Step 6: Update every `new FoleyConfig(...)` call site**

There are 4 sites (parallel to the M12/M13 plan's note):
- `FoleyConfig.java` itself (`DEFAULT`, `ALL`, `NONE`).
- 2 in `FoleyClientGameTest.java` (the M7 voice-test and the M8/M9 animals test, around lines 271 and 291).
- The test in `FoleyConfigTest.java`.

Use the withers or update each call. The 2 in `FoleyClientGameTest` chain `.withMinecarts(true).withDoors(true).withChests(true).withRedstone(true).withBoats(true).withSculk(true)` after the existing `withBirds(true)` to keep diff minimal.

- [ ] **Step 7: Run the Java unit tests**

Run: `cd mod && ./gradlew test`
Expected: pass.

- [ ] **Step 8: Commit**

```bash
git add mod/src/main/java/io/github/cumsauce/foley/config/FoleyConfig.java mod/src/test/java/io/github/cumsauce/foley/config/FoleyConfigTest.java
git commit -m "feat(config): add six active-structures toggles"
```

---

## Task 5: `Replacements.replace` learns the M21 events

**Files:**
- Modify: `mod/src/main/java/io/github/cumsauce/foley/audio/Replacements.java`

- [ ] **Step 1: Define the M21 event sets and skip rules**

Read the existing `Replacements` class. Add new `Set<Identifier>` fields parallel to `RAIN`, `THUNDER`, `WATER`, `LAVA`, `FIRE`, `DRIP_*`:

```java
private static final Set<Identifier> DOOR_WOOD = Set.of(
        SoundEvents.WOODEN_DOOR_OPEN.location(), SoundEvents.WOODEN_DOOR_CLOSE.location(),
        SoundEvents.WOODEN_TRAPDOOR_OPEN.location(), SoundEvents.WOODEN_TRAPDOOR_CLOSE.location());
private static final Set<Identifier> DOOR_IRON = Set.of(
        SoundEvents.IRON_DOOR_OPEN.location(), SoundEvents.IRON_DOOR_CLOSE.location(),
        SoundEvents.IRON_TRAPDOOR_OPEN.location(), SoundEvents.IRON_TRAPDOOR_CLOSE.location());
private static final Set<Identifier> CHEST = Set.of(
        SoundEvents.CHEST_OPEN.location(), SoundEvents.CHEST_CLOSE.location(),
        SoundEvents.TRAPPED_CHEST_OPEN.location(), SoundEvents.TRAPPED_CHEST_CLOSE.location());
private static final Set<Identifier> SHULKER = Set.of(
        SoundEvents.SHULKER_BOX_OPEN.location(), SoundEvents.SHULKER_BOX_CLOSE.location());
private static final Set<Identifier> MINECART = Set.of(
        SoundEvents.MINECART_INSIDE.location(), SoundEvents.MINECART_RIDING.location());
// (Confirm the exact identifier names; the implementer checks `SoundEvents`'s API for
// the 1.21.x set of minecart, door, chest, shulker events.)
```

(For some of these, the existing `Replacements` might already skip them via the `Layers` path. The implementer is careful: doors and chests are *not* `SoundType`-keyed, so they don't go through `Layers`; they need their own `Set<Identifier>` arm.)

- [ ] **Step 2: Add the dispatcher arm in `replace`**

```java
if (DOOR_WOOD.contains(id) || DOOR_IRON.contains(id)) {
    if (!FoleyClient.wants(FoleyConfig::doors)) return false;
    return counted();
}
if (CHEST.contains(id) || SHULKER.contains(id)) {
    if (!FoleyClient.wants(FoleyConfig::chests)) return false;
    return counted();
}
if (MINECART.contains(id)) {
    if (!FoleyClient.wants(FoleyConfig::minecarts)) return false;
    return counted();
}
```

For each skipped event, the `ActiveStructures.tick` (Task 6) plays Foley's layer/sound at the same position. The Redstone and Sculk events are *not* in the game sound set directly (redstone state changes don't fire a per-state-change sound in vanilla); the M21 work adds them as Foley-only triggered events. The Boat ambient doesn't have a vanilla analog; M21 adds it as a Foley-only bed.

- [ ] **Step 3: Add a `counted` wrapper around a new `FoleyClient.countReplaced()` if needed** (the existing `counted()` is fine).

- [ ] **Step 4: Compile and run the existing tests**

Run: `cd mod && ./gradlew test`
Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add mod/src/main/java/io/github/cumsauce/foley/audio/Replacements.java
git commit -m "refactor(replacements): skip M21 entity sounds when their toggle is on"
```

---

## Task 6: `ActiveStructures` controller (the bulk of M21)

**Files:**
- Create: `mod/src/main/java/io/github/cumsauce/foley/audio/ActiveStructures.java`

**Interfaces:**

```java
public final class ActiveStructures {
    /** Per-tick scan cadence (10 ticks = 0.5 s). */
    private static final int SAMPLE_EVERY = 10;
    /** How far the scan looks for rails, doors, chests, boats, sculk, etc. (blocks). */
    private static final int RADIUS = 16;
    /** Max boats with a lapping bed (matches `FoleyBeds.MOST`). */
    private static final int MAX_BOATS = 2;

    public ActiveStructures(NativeBridge bridge);
    public void tick(Minecraft minecraft);
    /** The M21 sounds Foley has played this session. For the gametest. */
    public int played();
    public BoatBed boats();
}
```

The tick:
1. **Rails** (within `RADIUS`): for each minecart on a rail, play `tap_wood` (a `Contact::Tap` on `Material::Wood`, speed 1.5) at the cart's position, every 0.4 s. Skip if the toggle is off.
2. **Powered rail**: when a minecart enters a powered-rail block, play `tap_metal` at 2× the level (the louder wheel impact the spec calls for).
3. **Minecart rush**: each minecart has a per-cart streaming bed (`FOLEY_BED_RAIN` is too heavy — a new small `BoatBed`-style class with a `BedSound<MinecartScene>` that renders a low rumble whose gain is proportional to the cart's `getDeltaMovement().horizontalDistance()`. The bed is at the cart's position; cap at `MAX_MINECART_BEDS = 2` to stay within the 8-stream budget (one of the 2 is the per-cart rush; the boat lapping beds share the same cap). When the cart is stationary the bed is silent; when it's moving it rushes louder.
3. **Doors**: nothing to tick — `Replacements.replace` skips the game event; the *layer* is `tap_wood` (or `tap_metal` for iron) at the door's position, played by `ActiveStructures.layerDoor` called from the `Replacements` skip path. **Action: rewire** — `Replacements.replace` calls `ActiveStructures.onDoorSkipped(sound)` which plays the Foley's tap.
4. **Chests/shulkers**: same pattern. `Replacements.replace` calls `ActiveStructures.onChestSkipped(sound)` to play `tap_wood` (chest) or `tap_wood` with a different envelope (shulker — the spec says "small puff of air," so use a `Bubble` one-shot from `FOLEY_PRIMITIVE_BUBBLE` with radius 4 mm).
5. **Redstone**: a tick listener on `ClientChunkEvents.CHUNK_LOAD` and `ClientTickEvents.END_CLIENT_TICK` (delta scan); for each block entity that's a redstone component (repeater, comparator), if its state changed since last scan, play `tap_stone` at low volume. This is a small registry keyed by `BlockPos`.
6. **Boats**: a per-boat bed (a `BoatBed` instance) that runs as long as the boat is in the world and within range. The bed renders bubble-primitive one-shots at a Poisson rate proportional to the boat's speed (or at 1/s when stationary). Cap at `MAX_BOATS`.
7. **Sculk sensors**: every 10 ticks, scan for `SCULK_SENSOR` and `SCULK_SHRIEKER` block entities. If a sensor's "last triggered" tick changed since last scan, call `playSculkRing` for the sensor and for each sculk block within 8 blocks. A `BlockEntity`-listener on `ClientTickEvents.END_CLIENT_TICK` polls the sensor's `LastVibrationFrequency` (1.21.x API) and triggers when it changes.

- [ ] **Step 1: Write `ActiveStructures.java`**

```java
package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.natives.NativeBridge;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Map;
import java.util.Set;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.vehicle.Minecart;
import net.minecraft.world.entity.vehicle.Boat;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.entity.SculkSensorBlockEntity;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/** Active structures (M21): minecarts, doors, chests, redstone, boats, sculk — wired to existing contact/primitive models
 * and the new sculk ring. Runs every 10 ticks. */
public final class ActiveStructures {
    private static final int SAMPLE_EVERY = 10;
    private static final int RADIUS = 16;

    private final NativeBridge bridge;
    private final BoatBed boats;
    /** Last minecart position per entity id, for distance-based playing. */
    private final Map<Integer, Vec3> lastCart = new HashMap<>();
    /** Last redstone component state per BlockPos, for delta detection. */
    private final Map<BlockPos, Integer> lastRedstone = new HashMap<>();
    /** Last sculk sensor vibration frequency per BlockPos, for trigger detection. */
    private final Map<BlockPos, Integer> lastSensor = new HashMap<>();
    private int played;
    private int ticks;

    public ActiveStructures(NativeBridge bridge) {
        this.bridge = bridge;
        this.boats = new BoatBed(bridge);
    }

    public int played() { return played; }
    public BoatBed boats() { return boats; }

    public void tick(Minecraft minecraft) {
        if (minecraft.level == null || minecraft.player == null) return;
        if (ticks++ % SAMPLE_EVERY != 0) return;
        // Minecarts on rails
        if (FoleyClient.wants(FoleyConfig::minecarts)) scanMinecarts(minecraft);
        // Boats lap at the hull
        if (FoleyClient.wants(FoleyConfig::boats)) boats.tick(minecraft);
        // Redstone state changes
        if (FoleyClient.wants(FoleyConfig::redstone)) scanRedstone(minecraft);
        // Sculk sensors
        if (FoleyClient.wants(FoleyConfig::sculk)) scanSculk(minecraft);
    }

    /** Called from {@link Replacements} when a door open/close event is skipped. Plays a creak at the door's position. */
    public void onDoorSkipped(Minecraft minecraft, net.minecraft.client.resources.sounds.SoundInstance sound, boolean iron) {
        if (minecraft.level == null || minecraft.player == null) return;
        if (minecraft.player.distanceToSqr(sound.getX(), sound.getY(), sound.getZ()) > 16 * 16) return;
        if (!FoleyClient.oneShot()) return;
        // A wooden door creaks (Contact::Tap on WOOD at 1.0 m/s); an iron door rings (Contact::Tap on METAL at 2.0 m/s).
        int material = iron ? Materials.METAL : Materials.WOOD;
        String event = Layers.CONTACTS[NativeBridge.CONTACT_TAP] + "_" + Materials.NAMES[material];
        minecraft.getSoundManager().play(new net.minecraft.client.resources.sounds.SimpleSoundInstance(
                Sounds.event(event), SoundSource.BLOCKS, 0.5f * FoleyClient.gain(), 1f,
                net.minecraft.client.resources.sounds.SoundInstance.createUnseededRandom(),
                sound.getX(), sound.getY(), sound.getZ()));
        played++;
    }

    /** Called from {@link Replacements} when a chest/shulker open/close is skipped. */
    public void onChestSkipped(Minecraft minecraft, net.minecraft.client.resources.sounds.SoundInstance sound, boolean shulker) {
        if (minecraft.level == null || minecraft.player == null) return;
        if (minecraft.player.distanceToSqr(sound.getX(), sound.getY(), sound.getZ()) > 16 * 16) return;
        if (!FoleyClient.oneShot()) return;
        if (shulker) {
            // A small puff: a Bubble primitive at 4 mm radius.
            short[] pcm = bridge.renderPrimitive(NativeBridge.PRIMITIVE_BUBBLE, new float[] {4.0f}, sound.getX() + sound.getX() != 0 ? 1L : 1L);
            // (Render to a temp buffer via the same path the addon uses; the implementer may use
            //  `bridge.renderOneShot(NativeBridge.SHOT_DRIP_INTO_WATER, 4.0f, ...)` as a stand-in for a small
            //  bubble pop, or add a one-shot variant. Cleanest: a new `FOLEY_SHOT_*` kind for the puff, but
            //  that requires another ABI bump. Use the existing drip-into-water with a small radius for now.)
            minecraft.getSoundManager().play(new net.minecraft.client.resources.sounds.SimpleSoundInstance(
                    Sounds.event("chest_open"), SoundSource.BLOCKS, 0.4f * FoleyClient.gain(), 1f,
                    net.minecraft.client.resources.sounds.SoundInstance.createUnseededRandom(),
                    sound.getX(), sound.getY(), sound.getZ()));
        } else {
            String event = Layers.CONTACTS[NativeBridge.CONTACT_TAP] + "_" + Materials.NAMES[Materials.WOOD];
            minecraft.getSoundManager().play(new net.minecraft.client.resources.sounds.SimpleSoundInstance(
                    Sounds.event(event), SoundSource.BLOCKS, 0.4f * FoleyClient.gain(), 1f,
                    net.minecraft.client.resources.sounds.SoundInstance.createUnseededRandom(),
                    sound.getX(), sound.getY(), sound.getZ()));
        }
        played++;
    }

    private void scanMinecarts(Minecraft minecraft) {
        Vec3 ear = minecraft.player.getEyePosition();
        Set<Integer> seen = new HashSet<>();
        for (Entity e : minecraft.level.entitiesForRendering()) {
            if (!(e instanceof Minecart)) continue;
            Vec3 pos = e.position();
            if (ear.distanceToSqr(pos) > RADIUS * RADIUS) continue;
            int id = e.getId();
            seen.add(id);
            Vec3 last = lastCart.get(id);
            BlockState rail = minecraft.level.getBlockState(e.blockPosition());
            boolean powered = rail.is(Blocks.POWERED_RAIL);
            // A wheel impact every ~0.4 s; faster on a powered rail.
            if (last == null || last.distanceTo(pos) > 0.3) {
                int material = rail.is(Blocks.POWERED_RAIL) ? Materials.METAL : Materials.WOOD;
                float volume = powered ? 0.7f : 0.35f;
                String event = Layers.CONTACTS[NativeBridge.CONTACT_TAP] + "_" + Materials.NAMES[material];
                minecraft.getSoundManager().play(new net.minecraft.client.resources.sounds.SimpleSoundInstance(
                        Sounds.event(event), SoundSource.BLOCKS, volume * FoleyClient.gain(), 1f,
                        net.minecraft.client.resources.sounds.SoundInstance.createUnseededRandom(),
                        pos.x, pos.y, pos.z));
                played++;
                lastCart.put(id, pos);
            }
        }
        lastCart.keySet().retainAll(seen);
    }

    private void scanRedstone(Minecraft minecraft) {
        // A simple delta scan: for every BlockEntity within range, encode its state as an int
        // and compare with last scan. The state-hash is intentionally coarse — repeater delay
        // and comparator mode are what matters for the click.
        Set<BlockPos> seen = new HashSet<>();
        for (Entity e : minecraft.level.entitiesForRendering()) {} // (no-op; we scan BlockEntities separately)
        for (int dx = -RADIUS; dx <= RADIUS; dx += 4) {
            for (int dz = -RADIUS; dz <= RADIUS; dz += 4) {
                int x = (int) Math.floor(minecraft.player.getX()) + dx;
                int z = (int) Math.floor(minecraft.player.getZ()) + dz;
                int top = minecraft.level.getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING, x, z);
                for (int y = minecraft.level.getMinY(); y <= top; y++) {
                    BlockPos pos = new BlockPos(x, y, z);
                    BlockEntity be = minecraft.level.getBlockEntity(pos);
                    if (be == null) continue;
                    Block b = be.getBlockState().getBlock();
                    if (b != Blocks.REPEATER && b != Blocks.COMPARATOR) continue;
                    seen.add(pos);
                    int state = be.getBlockState().getValue(net.minecraft.world.level.block.RedstoneWireBlock.POWER) * 17
                            + (b == Blocks.COMPARATOR
                                ? be.getBlockState().getValue(net.minecraft.world.level.block.ComparatorBlock.MODE).ordinal() * 7
                                : 0);
                    Integer last = lastRedstone.get(pos);
                    if (last != null && last != state) {
                        // A click: tap_stone at 0.18 gain.
                        String event = Layers.CONTACTS[NativeBridge.CONTACT_TAP] + "_" + Materials.NAMES[Materials.STONE];
                        minecraft.getSoundManager().play(new net.minecraft.client.resources.sounds.SimpleSoundInstance(
                                Sounds.event(event), SoundSource.BLOCKS, 0.18f * FoleyClient.gain(), 1f,
                                net.minecraft.client.resources.sounds.SoundInstance.createUnseededRandom(),
                                pos.getX() + 0.5, pos.getY() + 0.5, pos.getZ() + 0.5));
                        played++;
                    }
                    lastRedstone.put(pos, state);
                }
            }
        }
        lastRedstone.keySet().retainAll(seen);
    }

    private void scanSculk(Minecraft minecraft) {
        // A sculk sensor triggers when its `LastVibrationFrequency` changes. Each trigger fires
        // a ring at the sensor and at every sculk block within 8 blocks.
        for (Entity e : minecraft.level.entitiesForRendering()) {}
        for (int dx = -RADIUS; dx <= RADIUS; dx += 4) {
            for (int dz = -RADIUS; dz <= RADIUS; dz += 4) {
                int x = (int) Math.floor(minecraft.player.getX()) + dx;
                int z = (int) Math.floor(minecraft.player.getZ()) + dz;
                int top = minecraft.level.getHeight(net.minecraft.world.level.levelgen.Heightmap.Types.MOTION_BLOCKING, x, z);
                for (int y = minecraft.level.getMinY(); y <= top; y++) {
                    BlockPos pos = new BlockPos(x, y, z);
                    BlockEntity be = minecraft.level.getBlockEntity(pos);
                    if (!(be instanceof SculkSensorBlockEntity sensor)) continue;
                    int last = sensor.getLastVibrationFrequency().orElse(0);
                    Integer prev = lastSensor.get(pos);
                    if (prev != null && prev != last) {
                        // Triggered. Play at the sensor and at each sculk block within 8.
                        triggerSculkRing(minecraft, pos);
                        for (int ddx = -8; ddx <= 8; ddx += 2) {
                            for (int ddz = -8; ddz <= 8; ddz += 2) {
                                for (int ddy = -4; ddy <= 4; ddy += 2) {
                                    BlockPos at = pos.offset(ddx, ddy, ddz);
                                    if (minecraft.level.getBlockState(at).is(Blocks.SCULK)) {
                                        triggerSculkRing(minecraft, at);
                                    }
                                }
                            }
                        }
                    }
                    lastSensor.put(pos, last);
                }
            }
        }
    }

    private void triggerSculkRing(Minecraft minecraft, BlockPos at) {
        if (!FoleyClient.oneShot()) return;
        if (minecraft.player.distanceToSqr(at.getX() + 0.5, at.getY() + 0.5, at.getZ() + 0.5) > 24 * 24) return;
        String event = "sculk_ring";
        minecraft.getSoundManager().play(new net.minecraft.client.resources.sounds.SimpleSoundInstance(
                Sounds.event(event), SoundSource.BLOCKS, 0.3f * FoleyClient.gain(), 1f,
                net.minecraft.client.resources.sounds.SoundInstance.createUnseededRandom(),
                at.getX() + 0.5, at.getY() + 0.5, at.getZ() + 0.5));
        played++;
    }
}
```

(The implementer fills the exact API names: `SculkSensorBlockEntity.getLastVibrationFrequency()` is 1.20.3+; `Minecart` is `AbstractMinecart` in some versions; the `BlockPos` hash in `RedstoneWireBlock.POWER` may differ. **Important**: the implementer must check the 1.21.x names before writing the code; this is a hand-typed outline.)

- [ ] **Step 2: Create `BoatBed.java` and `BoatScene.java`**

`BoatScene` is a `record` with `params() : float[] { radiusMm, rate }`. `BoatBed` keeps a small list of active boats (capped at `MAX_BOATS`), creates a per-boat `BedSound<BoatScene>` (using the existing `BedSound.place` pattern), and renders `bubble` primitives per tick at a Poisson rate proportional to the boat's speed. The bed is *not* a stream; it's a one-shot per bubble. Use the existing `Bridge.renderPrimitive(PRIMITIVE_BUBBLE, [radius], seed)` per bubble.

```java
// BoatScene.java
public record BoatScene(float radiusMm) implements Scene {
    public float[] params() { return new float[] {radiusMm}; }
}
// BoatBed.java — small manager: 1..MAX_BOATS active BedSounds, each playing bubble primitives per tick.
```

(Boat lapping as a *placed bed* per boat is the spec; the bed "plays continuously," but the actual audio is per-bubble one-shots, not a stream. The implementer can choose: a true stream that mixes bubble primitives in real time, or a `tick()` that plays a `renderOneShot(BUBBLE, radius)` per bubble. The latter is simpler and fits the 8-stream budget better — at most 2 streams, but if a boat's bubbles are one-shots, the streams can be 0 and only the game's static sources are used. The spec is ambiguous; the plan documents the call as the implementer's choice.)

- [ ] **Step 2.1: Create `MinecartBed.java` and `MinecartScene.java`**

`MinecartScene` is a `record(float speed)` whose `params()` returns `[speed]`. `MinecartBed` keeps a list of `BedSound<MinecartScene>` (capped at `MAX_MINECART_BEDS = 2` total) and ticks once per scan. Each bed renders a low rumble whose gain is proportional to the cart's `getDeltaMovement().horizontalDistance()`. The bed's `setScene(...)` is called every scan with the cart's current speed; the bed is at the cart's world position. When the cart's speed is 0, the bed is silent (gain 0). The bed uses an existing primitive kind — a new `FOLEY_PRIMITIVE_RUMBLE` would be ideal but the spec doesn't define it; **the implementer reuses `FOLEY_PRIMITIVE_NOISE` centred at 80 Hz with Q 0.5, gain = `min(1, speed * 4)`** (sleek rush above 4 m/s). The "rush" sound is a low rumble that the bed plays continuously while the cart is moving; the wheel impacts in `scanMinecarts` are the per-impact transients.

```java
// MinecartScene.java
public record MinecartScene(float speed) implements Scene {
    public float[] params() { return new float[] {speed}; }
}
// MinecartBed.java — small manager: 1..2 active BedSounds for nearby minecarts.
```

- [ ] **Step 3: Compile**

Run: `cd mod && ./gradlew compileJava`
Expected: pass. (Compile errors from `Replacements` calling `ActiveStructures.onDoorSkipped`/`onChestSkipped` are addressed in Task 7.)

- [ ] **Step 4: Commit**

```bash
git add mod/src/main/java/io/github/cumsauce/foley/audio/ActiveStructures.java mod/src/main/java/io/github/cumsauce/foley/audio/BoatBed.java mod/src/main/java/io/github/cumsauce/foley/audio/BoatScene.java
git commit -m "feat(audio): add ActiveStructures controller for M21"
```

---

## Task 7: Wire `Replacements.replace` to `ActiveStructures` and `FoleyClient`

**Files:**
- Modify: `mod/src/main/java/io/github/cumsauce/foley/audio/Replacements.java`
- Modify: `mod/src/main/java/io/github/cumsauce/foley/FoleyClient.java`

- [ ] **Step 1: In `Replacements.replace`, route the door and chest events to `ActiveStructures`**

```java
if (DOOR_WOOD.contains(id) || DOOR_IRON.contains(id)) {
    if (!FoleyClient.wants(FoleyConfig::doors)) return false;
    FoleyClient.activeStructures().onDoorSkipped(minecraft, sound, DOOR_IRON.contains(id));
    return counted();
}
if (CHEST.contains(id) || SHULKER.contains(id)) {
    if (!FoleyClient.wants(FoleyConfig::chests)) return false;
    FoleyClient.activeStructures().onChestSkipped(minecraft, sound, SHULKER.contains(id));
    return counted();
}
if (MINECART.contains(id)) {
    if (!FoleyClient.wants(FoleyConfig::minecarts)) return counted();
}
```

- [ ] **Step 2: In `FoleyClient.java`, instantiate `ActiveStructures`**

```java
private static @Nullable ActiveStructures activeStructures;
public static @Nullable ActiveStructures activeStructures() { return activeStructures; }
```

In `onInitializeClient`, after the `Life` and `CaveController` setup:

```java
ActiveStructures active = new ActiveStructures(loaded);
activeStructures = active;
ClientTickEvents.END_CLIENT_TICK.register(active::tick);
```

- [ ] **Step 3: Compile**

Run: `cd mod && ./gradlew compileJava`
Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add mod/src/main/java/io/github/cumsauce/foley/audio/Replacements.java mod/src/main/java/io/github/cumsauce/foley/FoleyClient.java
git commit -m "feat: wire ActiveStructures into Replacements and FoleyClient"
```

---

## Task 8: sounds.json entries

**Files:**
- Modify: `mod/src/main/resources/assets/foley/sounds.json`

- [ ] **Step 1: Add 8 new event entries**

For each of `door_wood`, `door_iron`, `chest_open`, `shulker_open`, `redstone_click`, `minecart_wheel`, `boat_lap`, `sculk_ring`, add a `sounds` array with 4 variants pointing to `foley:generated/<event>_<0..3>`.ogg. The variants are never loaded (these are one-shots rendered to a buffer by the existing `GeneratedSounds.render` path, the way `drip_3` is). Look at the existing `bird_3` or `crickets_2` entry as a template.

```json
"door_wood": { "sounds": [ "foley:generated/door_wood_0", ..., "foley:generated/door_wood_3" ] },
"door_iron": { "sounds": [ ... ] },
"chest_open": { "sounds": [ ... ] },
"shulker_open": { "sounds": [ ... ] },
"redstone_click": { "sounds": [ ... ] },
"minecart_wheel": { "sounds": [ ... ] },
"boat_lap": { "sounds": [ ... ] },
"sculk_ring": { "sounds": [ ... ] }
```

- [ ] **Step 2: Make `GeneratedSounds` and `AddonSounds` render them**

In `GeneratedSounds.render`, add cases for the door/chest/click/wheel entries (these are just placeholders for the actual one-shots — the real M21 sounds are produced by `ActiveStructures` directly through `bridge.renderContact(...)` and `bridge.renderOneShot(SHOT_SCULK_RING, ...)`. The sounds.json entries exist so the *game's* event name resolves when called from `Replacements`. The variant buffers are unused. Keep the `default -> life(bridge, model, variant, contact(bridge, model, variant));` line as the catch-all — the door/chest/wheel events are NOT routed through `GeneratedSounds`; they are rendered by `ActiveStructures` via `bridge.renderContact` directly and played by a `SimpleSoundInstance(event, ...)` with the event being one of these entries.

So `sounds.json` just needs the entries; the events exist for `Sounds.event("door_wood")` to return a `SoundEvent` that the `SimpleSoundInstance` constructor can use. No rendering path is needed in `GeneratedSounds`.

- [ ] **Step 3: Commit**

```bash
git add mod/src/main/resources/assets/foley/sounds.json
git commit -m "feat(sounds): add M21 event entries to sounds.json"
```

---

## Task 9: Gametest — M21 sounds fire and toggle off silences them

**Files:**
- Modify: `mod/src/gametest/java/io/github/cumsauce/foley/gametest/FoleyClientGameTest.java`

- [ ] **Step 1: Update existing `FoleyConfig` call sites**

The two `FoleyClient.apply(new FoleyConfig(...))` calls now need 20 booleans + the voices long. The 6 new toggles default to `true` (off would be against the spec). The implementer uses `.withMinecarts(true).withDoors(true).withChests(true).withRedstone(true).withBoats(true).withSculk(true)` after the existing chain.

- [ ] **Step 2: Add an M21 test step after the existing mob-voice tests**

After the "M8/M9 animals" test (~line 349) and before the settings UI test, add a sequence:

1. Enable M21: `FoleyClient.apply(...all the new toggles on...)`.
2. Place a rail and a minecart: `summon minecart ~3 ~ ~`, `setblock ~3 ~-1 ~ rail`. Wait up to 10 s for `FoleyClient.activeStructures().played() > 0`.
3. Place a wooden door: `setblock ~6 ~ ~ oak_door[facing=north,open=true]`. Wait for the door event to play (use the existing `FoleyClient.replaced()` counter; the door open event is replaced and Foley's tap plays).
4. Place a chest: `setblock ~8 ~ ~ chest`. Open it (`/setblock ~8 ~ ~ chest` with the player's interaction simulated by a sound event `minecraft:block.chest.open` played at the position; the gametest's `setblock` doesn't trigger a chest open, so use `mc.getSoundManager().play(new SimpleSoundInstance(SoundEvents.CHEST_OPEN, ...))` directly).
5. Place a sculk sensor: `setblock ~ ~-1 ~ sculk_sensor`. Trigger it (play a footstep near the sensor; in the gametest, the easiest is to `summon zombie ~ ~1 ~` and let it walk).
6. After 30 s, assert `FoleyClient.activeStructures().played() >= 4` (minecart, door, chest, sculk).
7. Toggle everything off: `FoleyClient.apply(...all the new toggles off...)`. Repeat step 4 (chest open). The replacement should be skipped; `FoleyClient.replaced()` should *not* increment; `FoleyClient.activeStructures().played()` should not increment.

- [ ] **Step 3: Run the gametest**

Run: `cd mod && ./gradlew runGametest`
Expected: pass; M21 sounds fire when on, silent when off.

- [ ] **Step 4: Commit**

```bash
git add mod/src/gametest/java/io/github/cumsauce/foley/gametest/FoleyClientGameTest.java
git commit -m "test(gametest): M21 active structures fire and toggle"
```

---

## Task 10: End-to-end build and listening test

- [ ] **Step 1: Run all unit tests**

Run: `cargo test --workspace` and `cd mod && ./gradlew test`.
Expected: all pass.

- [ ] **Step 2: Build the jar**

Run: `cargo xtask natives --target host && cd mod && ./gradlew build`
Expected: jar built; ABI version 20.

- [ ] **Step 3: Listen in the running client**

`cd mod && ./gradlew runClient`. Place rails, a minecart, doors (wood and iron), a chest, a shulker box, a boat, and a sculk sensor. Trigger each:
- **Minecart:** ride a minecart on a rail. The wheel impacts should be heard at the right cadence; faster on a powered rail.
- **Doors:** open a wooden door (creak), an iron door (ring), a wooden trapdoor. Toggle *Doors* off in the settings; the game plays its own sound again.
- **Chests:** open a chest (puff), a trapped chest (redstone click + puff), a shulker box.
- **Redstone:** place a repeater and a comparator. Make a clock circuit (rapid redstone updates) — a faint click should be heard.
- **Boats:** place a boat in still water (gentle lapping); move it (faster lapping).
- **Sculk:** place a sculk sensor and walk near it. The ring should be heard at the sensor and at every nearby sculk block.

## Review Focus

These inputs/failure modes are implied by the spec but no test exercises them; they go here once, and the tests that pin them live in the listed task:

- **Per-tick scan budget.** `ActiveStructures` scans `O(RADIUS² × height)` blocks for redstone and sculk. At `RADIUS = 16`, that's `33² × ~64 = ~70 000` blocks every 10 ticks. Pin in Task 6 by adding an `assertTrue(playTime < 5)` (or `ticks < 10` for the scan) — the implementer times the scan and confirms it stays under 5 ms on a typical client. If it doesn't, drop the step from 4 to 8 (in `scanRedstone`'s `dx` loop) and add a Task 6.1 step to confirm.
- **Boat bed cap.** A 2-boat cap on `FoleyBeds.MOST` means a third nearby boat is silent. The spec doesn't say what to do; the plan says drop the boat's bed. Pin in Task 9 by spawning 3 boats and asserting only 2 lap.
- **Sculk trigger false positives.** `SculkSensorBlockEntity.getLastVibrationFrequency()` may be `Optional<Integer>` (1.21.x) or a different method name. If the implementer picks the wrong one, every tick counts as a "trigger" and the sculk ring fires constantly. Pin in Task 9 by running the test with a sculk sensor and no triggers, and asserting `played()` for sculk is 0.
- **Minecart wheel impact on every frame.** A stationary minecart on a rail with a misbehaving delta check would fire every tick. The plan's `distanceTo(pos) > 0.3` guard handles this; pin in Task 9 by placing a stationary minecart on a rail and asserting no impact in 5 s.
- **Sculk block fan-out.** A sculk sensor with 200 sculk blocks within 8 blocks would fire 201 one-shots. The plan's one-shot cap (`FoleyClient.oneShot()`) and the `played++` counter would mask this, but the audio would still spike. Pin by checking `played()` is bounded to 50 in a single trigger event.
- **Toggle round-trip.** A config with all six toggles off should round-trip through `FoleyConfig.fromJson(...).toJson()`. Pin in Task 4 by extending the `FoleyConfigTest` to assert each toggle's value survives a round-trip.
- **Chest + trapped chest sounds.** A trapped chest open in vanilla plays both the chest open and a redstone click. M21's `onChestSkipped` is called once per event. The redstone click is fired separately by `scanRedstone` if the comparator/repeater inside changes. Pin in Task 9 by opening a trapped chest and asserting both events play.
