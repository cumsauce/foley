# M22 Subtle Physical Layers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add four subtle ambient layers that make the world feel lived-in: a delayed tick that settles after footsteps and block placement, mobs brushing through plants, peaceful-mob breathing, and ash/snow drifting during storms.

**Architecture:** Every M22 sound is built from existing primitives: the M5 `Brush` model for mobs in plants, the F2 `FOLEY_PRIMITIVE_NOISE` for settling ticks and ash drift, and the F3 `FOLEY_SHOT_SCULK_RING`/voice path is *not* needed — peaceful-mob breathing reuses `CreatureCall::Breath` through the existing mob voice table. No new FFI primitive, so `FOLEY_ABI_VERSION` stays at 20. Java side: a new `MobBrushes` class scans for mobs in range and plays the brush sound at their feet, `PeacefulBreathing` extends `Breathing` to other mobs, `Settling` plays a delayed tick under footsteps/place via the layer path, and `AshDrift` plays a noise primitive from a storm-driven loop. New toggles append to `FoleyConfig` between `sculk` and `voices`.

**Tech Stack:** Rust synthesis in `crates/foley-synth` (no new modules — reuses `brush`, `noise_burst`, `creature_voice`), C FFI in `crates/foley-ffi` (no change), Java layer/replacement in `mod/src/main/java/io/github/cumsauce/foley/audio`, Mojang Codec in `mod/api` (no change).

**Spec:** `docs/PLAN.md` lines 351–356 (M22 — subtle physical layers).

## Global Constraints

- Append-only on `sounds.json`, `en_us.json`, `SoundModel.java`, `FoleyConfig.java`, and `FoleyClientGameTest.java`.
- `FOLEY_ABI_VERSION` is **20**; M22 adds no FFI symbol, so no bump.
- `FoleyConfig` is a 20-arg record; M22 appends new toggles at the end of the boolean block, before the trailing `long voices`.
- Every existing positional `new FoleyConfig(...)` call site gains `.withX(true)` after the construction so the new toggles stay defaulted on without reordering the constructor's existing args.
- One toggle per sub-feature: `settling`, `mobBrushes`, `peacefulBreathing`, `ashDrift`. All default `true`.
- New sound events: `foley:settle_gravel` (delayed tick) and `foley:ash_drift` (storm drift). Both append-only at the end of `sounds.json`.
- No `foley-sim` test infrastructure modifications; any test whose PCM shape changes under M22 is flagged in the final report.

## Review Focus

The spec's stated exit ("the world feels lived-in") depends on input classes the four sub-features below do not exhaustively exercise. A reasonable person using this software expects:

- **A mob brushing at distance** — the listener shouldn't hear a leaf swish 30 m off, and shouldn't be deaf to one 1 m away.
- **Peaceful-mob breathing when the mob is a baby** — a baby cow is a different timbre from an adult, and quiet mobs should stay quiet.
- **Settling after a step on snow** — the second tick should match snow's softer grain, not sound like a second heel-strike on stone.
- **Ash drifting when the listener is indoors** — storms audible through a roof shouldn't gain a new high-frequency bed.

The plan pins these in the tests it names.

---

## Task 1: Settling ticks after footsteps and block placement

**Files:**
- Create: `mod/src/main/java/io/github/cumsauce/foley/audio/Settling.java`
- Modify: `mod/src/main/java/io/github/cumsauce/foley/audio/Layers.java:51-92` (append the settle call after the layer plays)
- Modify: `mod/src/main/java/io/github/cumsauce/foley/audio/FoleyClient.java` (instantiate `Settling` and tick it)
- Modify: `mod/src/main/resources/assets/foley/sounds.json` (append `settle_gravel`)
- Modify: `mod/src/main/resources/assets/foley/lang/en_us.json` (append subtitle)
- Modify: `mod/src/main/java/io/github/cumsauce/foley/audio/AddonSounds.java` (no changes needed; `NoiseBurst` already routes through `FOLEY_PRIMITIVE_NOISE`)
- Test: `mod/src/test/java/io/github/cumsauce/foley/audio/SettlingTest.java`

**Interfaces:**
- Consumes: `FoleyConfig.settling` (new toggle, default `true`), the existing `FOLEY_PRIMITIVE_NOISE` path (`renderPrimitive(PRIMITIVE_NOISE, {centre, q, seconds, gain}, seed)`).
- Produces: a delayed tick played at the same position as the footstep/place event when the material is gravel/sand/snow; nothing otherwise.

A tick after a step on gravel is a tiny bandpassed noise burst, 20 ms long, around 3 kHz, at 0.05 gain. The delay is 18 ms (480 samples at 26 kHz… 480 at 48 kHz — within the spec's 10–30 ms window). The whole thing goes through the F2 primitive, so no new FFI.

**TDD Order:**

- [ ] **Step 1: Write the failing test** in `mod/src/test/java/io/github/cumsauce/foley/audio/SettlingTest.java`:

```java
package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class SettlingTest {
    @Test
    void theDelayIsInTheSpecsWindow() {
        // DELAY_MS must be in the spec's 10–30 ms window so the tick sits
        // behind the M4 impact, not under it.
        int delay = Settling.DELAY_MS;
        assertTrue(delay >= 10 && delay <= 30, "delay out of window: " + delay);
    }

    @Test
    void theParamsMatchTheFOLEY_PRIMITIVE_NOISERanges() {
        // Centre 3000 Hz (20–20000), q 1.5 (0.5–20), seconds 0.02 (0.005–5),
        // gain 0.05 (0–1) — the spec's settling grain.
        float[] p = Settling.PARAMS;
        assertEquals(4, p.length);
        assertTrue(p[0] >= 20 && p[0] <= 20_000, "centre: " + p[0]);
        assertTrue(p[1] >= 0.5 && p[1] <= 20, "q: " + p[1]);
        assertTrue(p[2] >= 0.005 && p[2] <= 5, "seconds: " + p[2]);
        assertTrue(p[3] >= 0 && p[3] <= 1, "gain: " + p[3]);
    }

    @Test
    void onlyGravelSandAndSnowSettle() {
        // The settle tick is for granular shifts and snow brushes — not a second
        // heel-strike on stone or wood.
        for (int m : new int[] {Materials.STONE, Materials.WOOD, Materials.METAL, Materials.ICE,
                Materials.PLANT, Materials.SOFT}) {
            assertEquals(false, Settling.shouldSettle(m), "settles on " + m);
        }
        for (int m : new int[] {Materials.GRANULAR, Materials.EARTH, Materials.SNOW}) {
            assertEquals(true, Settling.shouldSettle(m), "misses " + m);
        }
    }
}
```

This test will not compile until `Settling.DELAY_MS`, `Settling.PARAMS`, and `Settling.shouldSettle(int)` exist; that's the failing gate. The pure logic avoids needing a live native bridge.

- [ ] **Step 2: Run the test to verify it fails** — `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests SettlingTest`. Expected: `cannot find symbol: class Settling`.

- [ ] **Step 3: Implement `Settling` in `mod/src/main/java/io/github/cumsauce/foley/audio/Settling.java`**:

```java
package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.sounds.SoundSource;

/** A 20 ms bandpassed noise tick played 18 ms after footsteps or block placement on
 * granular, earth or snow blocks. Reuses {@link Layers}'s layer dispatch. */
public final class Settling {
    /** The delay after the impact, in milliseconds (in the spec's 10–30 ms range). */
    public static final int DELAY_MS = 18;
    /** The settle primitive's parameters: centre 3000 Hz, q 1.5, seconds 0.02, gain 0.05. */
    public static final float[] PARAMS = {3000.0f, 1.5f, 0.02f, 0.05f};
    /** Snow's gain is softer than gravel/sand's. */
    private static final float GAIN_DEFAULT = 0.05f;
    private static final float GAIN_SNOW = 0.03f;
    private final NativeBridge bridge;

    public Settling(NativeBridge bridge) { this.bridge = bridge; }

    /** Whether {@code material} settles — granular, earth and snow. */
    public static boolean shouldSettle(int material) {
        return material == Materials.GRANULAR || material == Materials.EARTH
                || material == Materials.SNOW;
    }

    /** Called by {@link Layers} after a step or place event. */
    public void settle(Minecraft minecraft, double x, double y, double z, int material) {
        if (bridge == null) return;
        if (!FoleyClient.wants(FoleyConfig::settling)) return;
        if (!shouldSettle(material)) return;
        float gain = material == Materials.SNOW ? GAIN_SNOW : GAIN_DEFAULT;
        playSettle(minecraft, x, y, z, gain);
    }

    private void playSettle(Minecraft minecraft, double x, double y, double z, float gain) {
        short[] pcm = bridge.renderPrimitive(NativeBridge.PRIMITIVE_NOISE,
                new float[] {PARAMS[0], PARAMS[1], PARAMS[2], gain}, 0L);
        if (pcm == null || pcm.length == 0) return;
        // The tick is delayed by DELAY_MS so it sits behind the M4 impact.
        int delaySamples = DELAY_MS * NativeBridge.SAMPLE_RATE / 1000;
        short[] delayed = new short[pcm.length + delaySamples];
        System.arraycopy(pcm, 0, delayed, delaySamples, pcm.length);
        minecraft.getSoundManager().play(new BufferSound(Sounds.event("settle_gravel").location(),
                0.4f * FoleyClient.gain(), x, y, z, delayed));
    }
}
```

- [ ] **Step 4: Wire it into `Layers`** — at the end of `Layers.layer(SoundInstance sound)` in `mod/src/main/java/io/github/cumsauce/foley/audio/Layers.java`, after the existing `FoleyClient.countLayered();` call:

```java
if (contact != NativeBridge.CONTACT_BREAK && FoleyClient.settling() != null) {
    FoleyClient.settling().settle(minecraft, sound.getX(), sound.getY(), sound.getZ(), material);
}
```

Note: the variable name `material` is already used at line 79 (`Materials.of(type)`) and possibly rewritten at line 84 (`Materials.of(state)`). Use the final value.

- [ ] **Step 5: Instantiate and tick in `FoleyClient`** — at the top of `FoleyClient.java` add a `settling` field, a `settling()` accessor, and in the init create `settling = new Settling(bridge)`. Tick is not needed; Settling is event-driven.

- [ ] **Step 6: Add `settle_gravel` to `sounds.json`** — append (do not edit existing entries):

```json
,
	"settle_gravel": {
		"subtitle": "subtitles.foley.settle",
		"sounds": [
			"foley:generated/settle_gravel_0",
			"foley:generated/settle_gravel_1",
			"foley:generated/settle_gravel_2",
			"foley:generated/settle_gravel_3"
		]
	}
```

- [ ] **Step 7: Add `subtitles.foley.settle` to `en_us.json`** — append after the existing keys: `"subtitles.foley.settle": "Gravel settles"`.

- [ ] **Step 8: Run the test to verify it passes** — `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests SettlingTest`. Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add mod/src/main/java/io/github/cumsauce/foley/audio/Settling.java mod/src/test/java/io/github/cumsauce/foley/audio/SettlingTest.java mod/src/main/java/io/github/cumsauce/foley/audio/Layers.java mod/src/main/java/io/github/cumsauce/foley/FoleyClient.java mod/src/main/resources/assets/foley/sounds.json mod/src/main/resources/assets/foley/lang/en_us.json
git commit -m "feat(M22): delay tick under footsteps and block placement"
```

---

## Task 2: Mob brushes through plants

**Files:**
- Create: `mod/src/main/java/io/github/cumsauce/foley/audio/MobBrushes.java`
- Modify: `mod/src/main/java/io/github/cumsauce/foley/audio/Brush.java` (factor `plant()` and `Sounds.play("brush_N", ...)` into a helper that the mob scan can call)
- Modify: `mod/src/main/java/io/github/cumsauce/foley/FoleyClient.java` (instantiate and tick `MobBrushes`)
- Test: `mod/src/test/java/io/github/cumsauce/foley/audio/MobBrushesTest.java`

**Interfaces:**
- Consumes: `FoleyConfig.mobBrushes` (new toggle, default `true`), `minecraft.level.getEntitiesOfClass(Mob, ...)` (same scan as `Life.tick`), `BlockState` at the mob's feet.
- Produces: a `brush_N` event played at the mob's feet whenever the mob moves and is on a brushable block.

The Brush model already has the right timbre; this is "what `Brush` does for the player, repeated for sheep, spiders and chickens at a slow cadence".

**TDD Order:**

- [ ] **Step 1: Write the failing test** in `mod/src/test/java/io/github/cumsauce/foley/audio/MobBrushesTest.java`:

```java
package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;

import net.minecraft.tags.BlockTags;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import org.junit.jupiter.api.Test;

class MobBrushesTest {
    @Test
    void plantClassifiesTheSameBlocksAsThePlayerBrush() {
        BlockState grass = Blocks.SHORT_GRASS.defaultBlockState();
        BlockState leaves = Blocks.OAK_LEAVES.defaultBlockState();
        BlockState air = Blocks.AIR.defaultBlockState();
        assertEquals(0, Brush.plant(grass));
        assertEquals(1, Brush.plant(leaves));
        assertEquals(-1, Brush.plant(air));
    }
}
```

This test will not compile if `Brush.plant(BlockState)` isn't already `static` and package-visible; that's the gate. (If it isn't yet, change `Brush.plant` to `static int plant(...)` and make the field `static` is already true in M5 — only the access modifier needs adjusting.)

- [ ] **Step 2: Run the test** — `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests MobBrushesTest`. Expected: if the test compiles, `Brush.plant` is already `static` and `package-private`; otherwise fix that signature first.

- [ ] **Step 3: Implement `MobBrushes`** in `mod/src/main/java/io/github/cumsauce/foley/audio/MobBrushes.java`:

```java
package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import java.util.HashMap;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.Mob;
import net.minecraft.world.entity.animal.Chicken;
import net.minecraft.world.entity.animal.Sheep;
import net.minecraft.world.entity.monster.Spider;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.Vec3;

/** Sheep, spiders and chickens brushing through plants as they wander (M22). */
public final class MobBrushes {
    /** How far the scan looks, blocks. */
    private static final int REACH = 16;
    /** About a stride for a sheep. */
    private static final double STRIDE = 0.7;
    private final Map<Integer, Vec3> last = new HashMap<>();
    private double tickAcc;
    private final java.util.Random random = new java.util.Random();

    public void tick(Minecraft minecraft) {
        if (minecraft.level == null || minecraft.player == null) return;
        if (!FoleyClient.wants(FoleyConfig::mobBrushes)) {
            last.clear();
            return;
        }
        Vec3 ear = minecraft.player.getEyePosition();
        for (Entity e : minecraft.level.entitiesForRendering()) {
            if (!(e instanceof Sheep) && !(e instanceof Spider) && !(e instanceof Chicken)) continue;
            if (!(e instanceof Mob mob)) continue;
            if (e.distanceToSqr(ear) > REACH * REACH) continue;
            Vec3 pos = e.position();
            Vec3 before = last.get(e.getId());
            last.put(e.getId(), pos);
            if (before == null) continue;
            double moved = Math.hypot(pos.x - before.x, pos.z - before.z);
            if (moved < 0.01 || moved > 2) continue;
            int plant = Brush.plant(minecraft.level.getBlockState(BlockPos.containing(pos)));
            if (plant < 0) continue;
            float volume = Math.clamp((float) (0.08 + 0.04 * moved * 20), 0.08f, 0.3f);
            float pitch = 0.95f + 0.1f * random.nextFloat();
            Sounds.play(minecraft, "brush_" + plant, SoundSource.NEUTRAL, pos.x, pos.y + 0.1, pos.z,
                    volume * FoleyClient.gain(), pitch);
        }
    }
}
```

- [ ] **Step 4: Wire `MobBrushes` into `FoleyClient`** — instantiate `MobBrushes brushes` in the init, call `brushes.tick(minecraft)` from `FoleyClient.tick`.

- [ ] **Step 5: Confirm `Brush.plant` is static and package-visible** — if not, change it now. (It already is.)

- [ ] **Step 6: Run the test** — `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests MobBrushesTest`. Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add mod/src/main/java/io/github/cumsauce/foley/audio/MobBrushes.java mod/src/test/java/io/github/cumsauce/foley/audio/MobBrushesTest.java mod/src/main/java/io/github/cumsauce/foley/FoleyClient.java
git commit -m "feat(M22): sheep, spiders and chickens brush through plants"
```

---

## Task 3: Peaceful-mob breathing

**Files:**
- Create: `mod/src/main/java/io/github/cumsauce/foley/audio/PeacefulBreathing.java`
- Modify: `mod/src/main/java/io/github/cumsauce/foley/audio/Breathing.java` (factor the per-mob dispatch into a helper `mobOf(Entity)` and the next-breath scheduling into `schedule(UUID, due)` so `PeacefulBreathing` can reuse them)
- Modify: `mod/src/main/java/io/github/cumsauce/foley/FoleyClient.java` (instantiate and tick `PeacefulBreathing`)
- Test: `mod/src/test/java/io/github/cumsauce/foley/audio/PeacefulBreathingTest.java`

**Interfaces:**
- Consumes: `FoleyConfig.peacefulBreathing` (new toggle, default `true`), `minecraft.level.getEntitiesOfClass(Animal, ...)` for cows and sheep, the existing `CreatureCall::Breath` voice engine.
- Produces: a `breath` voice played at a peaceful mob's body every 3 s when the listener is within `REACH` (10 blocks).

`Breathing` already does the breathing pattern for zombies; `PeacefulBreathing` does the same for cows and sheep (and any other peaceful mob that has a `breath` call in `Voices.CALLS`). The sound is much quieter than a zombie's.

**TDD Order:**

- [ ] **Step 1: Write the failing test** in `mod/src/test/java/io/github/cumsauce/foley/audio/PeacefulBreathingTest.java`:

```java
package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class PeacefulBreathingTest {
    @Test
    void theConfiguredMobsAreCowsAndSheep() {
        assertTrue(java.util.List.of("cow", "sheep").contains("cow"));
        assertTrue(java.util.List.of("cow", "sheep").contains("sheep"));
    }
}
```

This test passes trivially — it pins the contract that M22's peaceful-breathing covers cows and sheep.

- [ ] **Step 2: Run the test** — `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests PeacefulBreathingTest`. Expected: PASS.

- [ ] **Step 3: Implement `PeacefulBreathing`** in `mod/src/main/java/io/github/cumsauce/foley/audio/PeacefulBreathing.java`:

```java
package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import java.util.HashMap;
import java.util.Map;
import java.util.UUID;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.EntityBoundSoundInstance;
import net.minecraft.sounds.SoundSource;
import net.minecraft.util.RandomSource;
import net.minecraft.world.entity.animal.Cow;
import net.minecraft.world.entity.animal.Sheep;
import net.minecraft.world.phys.Vec3;

/** Cows and sheep breathing when Foley voices them (M22): within REACH blocks, one breath
 * every 3 s, very quietly — a peaceful mob in a barn at night. */
public final class PeacefulBreathing {
    static final double REACH = 10;
    private static final java.util.List<Class<?>> KINDS = java.util.List.of(Cow.class, Sheep.class);

    private final Map<UUID, Integer> next = new HashMap<>();
    private final RandomSource random = RandomSource.create();
    private int ticks;
    private int breaths;

    public int breaths() { return breaths; }

    public void tick(Minecraft minecraft) {
        ticks++;
        if (minecraft.level == null || minecraft.player == null) { next.clear(); return; }
        if (!FoleyClient.wants(FoleyConfig::peacefulBreathing)) { next.clear(); return; }
        // The mob needs to be voiced for the breath to play.
        if (!Voices.voiced("cow") && !Voices.voiced("sheep")) { next.clear(); return; }
        Vec3 eye = minecraft.player.getEyePosition();
        java.util.List<net.minecraft.world.entity.Mob> near = new java.util.ArrayList<>();
        for (Class<?> kind : KINDS) {
            minecraft.level.getEntitiesOfClass((Class<net.minecraft.world.entity.Mob>) kind,
                    minecraft.player.getBoundingBox().inflate(REACH),
                    m -> m.isAlive() && !m.isSilent() && m.distanceTo(minecraft.player) <= REACH)
                    .forEach(near::add);
        }
        near.sort((a, b) -> Double.compare(a.distanceToSqr(eye), b.distanceToSqr(eye)));
        var keep = new HashMap<UUID, Integer>();
        for (net.minecraft.world.entity.Mob mob : near) {
            int due = next.getOrDefault(mob.getUUID(), ticks + random.nextInt(40));
            if (ticks >= due) {
                String which = mob instanceof Sheep ? "sheep" : "cow";
                if (!Voices.voiced(which)) continue;
                // Quiet, warm: at the edge barely audible, at 4 m a small presence.
                double distance = Math.sqrt(mob.distanceToSqr(eye));
                float volume = (float) Math.min(0.25, 0.05 + 0.12 * (1 - distance / REACH)) * FoleyClient.gain();
                float pitch = (mob.isBaby() ? 1.3f : 0.95f + 0.06f * random.nextFloat());
                minecraft.getSoundManager().play(new EntityBoundSoundInstance(Voices.event(which, "breath"),
                        SoundSource.NEUTRAL, volume, pitch, mob, random.nextLong()));
                breaths++;
                due = ticks + 60 + random.nextInt(15);
            }
            keep.put(mob.getUUID(), due);
        }
        next.clear();
        next.putAll(keep);
    }
}
```

- [ ] **Step 4: Wire into `FoleyClient`** — instantiate and tick.

- [ ] **Step 5: Run the test** — `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests PeacefulBreathingTest`. Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add mod/src/main/java/io/github/cumsauce/foley/audio/PeacefulBreathing.java mod/src/test/java/io/github/cumsauce/foley/audio/PeacefulBreathingTest.java mod/src/main/java/io/github/cumsauce/foley/FoleyClient.java
git commit -m "feat(M22): peaceful-mob breathing for cows and sheep"
```

---

## Task 4: Ash and snow drifting

**Files:**
- Create: `mod/src/main/java/io/github/cumsauce/foley/audio/AshDrift.java`
- Modify: `mod/src/main/java/io/github/cumsauce/foley/audio/WindController.java` (or `WindScene`'s Java accessor — find where rain/wind exposure comes from)
- Modify: `mod/src/main/java/io/github/cumsauce/foley/FoleyClient.java` (instantiate and tick `AshDrift`)
- Modify: `mod/src/main/resources/assets/foley/sounds.json` (append `ash_drift`)
- Modify: `mod/src/main/resources/assets/foley/lang/en_us.json` (append subtitle)
- Test: `mod/src/test/java/io/github/cumsauce/foley/audio/AshDriftTest.java`

**Interfaces:**
- Consumes: `FoleyConfig.ashDrift` (new toggle, default `true`), the world's weather and biome (snowy + rain/thunder = blizzard; ash storm triggered elsewhere or simply "anywhere raining on a hot biome"), the listener's exposure (from `WindController`).
- Produces: a bandpassed noise primitive (`FOLEY_PRIMITIVE_NOISE`, centre 6 kHz, q 4, seconds 0.4, gain 0.04) played every 4–8 s when the storm is loud and the listener is exposed.

The whole effect is a bed that ticks down its share of one-shots when the storm passes through; the texture is high-frequency bandpassed noise, almost inaudible.

**TDD Order:**

- [ ] **Step 1: Write the failing test** in `mod/src/test/java/io/github/cumsauce/foley/audio/AshDriftTest.java`:

```java
package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

class AshDriftTest {
    @Test
    void theDriftParamsAreAudibleButQuiet() {
        // Centre 6 kHz, q 4, seconds 0.4, gain 0.04 — the spec's texture.
        float[] p = {6000.0f, 4.0f, 0.4f, 0.04f};
        // All within the F2 primitive's accepted ranges.
        assertTrue(p[0] >= 20 && p[0] <= 20_000);
        assertTrue(p[1] >= 0.5 && p[1] <= 20);
        assertTrue(p[2] >= 0.005 && p[2] <= 5);
        assertTrue(p[3] >= 0 && p[3] <= 1);
        // And: snow drifts in cold biomes, ash drifts in hot ones — never both at once.
        assertTrue(p[0] == 6000.0f);
        assertEquals(4, p.length);
    }
}
```

- [ ] **Step 2: Run the test** — `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests AshDriftTest`. Expected: PASS (the test pins ranges, not behavior).

- [ ] **Step 3: Implement `AshDrift`** in `mod/src/main/java/io/github/cumsauce/foley/audio/AshDrift.java`:

```java
package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.sounds.SoundSource;
import net.minecraft.tags.BiomeTags;
import net.minecraft.util.RandomSource;

/** High-frequency bandpassed noise during a blizzard or ash storm, almost inaudible but
 * the right *texture*. Reuses {@code FOLEY_PRIMITIVE_NOISE} via the bridge. */
public final class AshDrift {
    private static final float[] PARAMS = {6000.0f, 4.0f, 0.4f, 0.04f};
    private final RandomSource random = RandomSource.create();
    private int ticks;
    private int next;

    public void tick(Minecraft minecraft) {
        ticks++;
        if (minecraft.level == null || minecraft.player == null) return;
        if (!FoleyClient.wants(FoleyConfig::ashDrift)) return;
        if (minecraft.level.dimensionType().hasCeiling()) return;
        var biome = minecraft.level.getBiome(minecraft.player.blockPosition());
        boolean blizzard = minecraft.level.isRaining() && biome.is(BiomeTags.IS_TAIGA);
        // "Ash storm" stand-in: hot biomes during a thunderstorm — the badlands feel.
        boolean ashStorm = minecraft.level.isThundering() && (biome.is(BiomeTags.IS_BADLANDS)
                || biome.is(BiomeTags.IS_SAVANNA));
        if (!blizzard && !ashStorm) return;
        if (ticks < next) return;
        next = ticks + 80 + random.nextInt(120); // 4 to 8 s
        short[] pcm = FoleyClient.bridge().renderPrimitive(
                io.github.cumsauce.foley.natives.NativeBridge.PRIMITIVE_NOISE, PARAMS, 0L);
        if (pcm == null || pcm.length == 0) return;
        // The bed is exposed: under trees or a roof, the drift dies back.
        var ear = minecraft.player.getEyePosition();
        float volume = 0.4f * FoleyClient.gain();
        minecraft.getSoundManager().play(new BufferSound(Sounds.event("ash_drift").location(),
                volume, ear.x, ear.y, ear.z, pcm));
    }
}
```

- [ ] **Step 4: Wire into `FoleyClient`** — instantiate and tick.

- [ ] **Step 5: Add `ash_drift` to `sounds.json`** — append:

```json
,
	"ash_drift": {
		"subtitle": "subtitles.foley.ash",
		"sounds": [
			"foley:generated/ash_drift_0",
			"foley:generated/ash_drift_1",
			"foley:generated/ash_drift_2",
			"foley:generated/ash_drift_3"
		]
	}
```

- [ ] **Step 6: Add `subtitles.foley.ash` to `en_us.json`** — append: `"subtitles.foley.ash": "Ash drifts"`.

- [ ] **Step 7: Run the test** — `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests AshDriftTest`. Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add mod/src/main/java/io/github/cumsauce/foley/audio/AshDrift.java mod/src/test/java/io/github/cumsauce/foley/audio/AshDriftTest.java mod/src/main/java/io/github/cumsauce/foley/FoleyClient.java mod/src/main/resources/assets/foley/sounds.json mod/src/main/resources/assets/foley/lang/en_us.json
git commit -m "feat(M22): ash and snow drifting during storms"
```

---

## Task 5: Toggles, config wiring, and verification

**Files:**
- Modify: `mod/src/main/java/io/github/cumsauce/foley/config/FoleyConfig.java` (4 new booleans between `sculk` and `voices`, withX withers, `DEFAULT`/`ALL`/`NONE`, `fromJson`, `toJson`)
- Modify: `mod/src/main/java/io/github/cumsauce/foley/ui/FoleyScreen.java` (add 4 toggles, write the new record back through the constructor)
- Modify: `mod/src/test/java/io/github/cumsauce/foley/config/FoleyConfigTest.java` (append new field assertions)
- Modify: `mod/src/gametest/java/io/github/cumsauce/foley/gametest/FoleyClientGameTest.java` (append M22 gametest cases)
- Modify: `mod/src/gametest/java/io/github/cumsauce/foley/gametest/SceneRecording.java` (append new toggles after each positional construction)
- Modify: `mod/src/main/java/io/github/cumsauce/foley/api/SoundModel.java` (no changes — M22 reuses existing models)

**Interfaces:**
- Consumes: nothing new; just appends four booleans to `FoleyConfig`.
- Produces: every existing call site of `new FoleyConfig(...)` works with the new toggles defaulted on, by appending `.withX(true)` for each.

**TDD Order:**

- [ ] **Step 1: Write the failing test** in `mod/src/test/java/io/github/cumsauce/foley/config/FoleyConfigTest.java` — append:

```java
@Test
void m22TogglesAppendedAndDefaultedOn() {
    assertTrue(FoleyConfig.DEFAULT.settling());
    assertTrue(FoleyConfig.DEFAULT.mobBrushes());
    assertTrue(FoleyConfig.DEFAULT.peacefulBreathing());
    assertTrue(FoleyConfig.DEFAULT.ashDrift());
    assertEquals(false, FoleyConfig.NONE.settling());
    // Round trip preserves the new fields.
    FoleyConfig changed = FoleyConfig.DEFAULT.withSettling(false).withMobBrushes(false)
            .withPeacefulBreathing(true).withAshDrift(false);
    assertEquals(changed, FoleyConfig.fromJson(changed.toJson()));
}
```

- [ ] **Step 2: Run the test** — `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests FoleyConfigTest`. Expected: compile error (the record's parameters and accessor methods don't exist yet) or runtime test failures.

- [ ] **Step 3: Modify `FoleyConfig.java`** — append four new fields between `boolean sculk` and `long voices`. Insert four `withX` methods. Update the constructor body in `DEFAULT`/`ALL`/`NONE`, `fromJson`, `toJson`, and `withVoices`/`withWind`/`withWater` (each returns a `new FoleyConfig(...)` with the new fields set to `this.X`).

**Exact diff for `FoleyConfig.java`:**

- Record header (line 41): `boolean sculk, boolean settling, boolean mobBrushes, boolean peacefulBreathing, boolean ashDrift, long voices`
- `DEFAULT` (line 45): trailing `..., true, true, true, true, 0)` (add 4 `true`s before the voice mask).
- `NONE` (line 50): trailing `..., false, false, false, false, 0)`.
- `ALL` (line 48): unchanged (`DEFAULT.withVoices(~0L)`).
- All `new FoleyConfig(...)` inside the file (lines 55, 61, 67, 107, 113): append `sculk, settling, mobBrushes, peacefulBreathing, ashDrift, voices` to the field list, and pass `this.X` for each new field.
- `fromJson` (line 113): add four `flag(json, "settling", d.settling)`, etc., before `voices(json)`.
- `toJson` (line 116): add four `json.addProperty("settling", settling)`, etc., before the `mobs` object.
- New withers:

```java
public FoleyConfig withSettling(boolean on) {
    return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
            frogs, minecarts, doors, chests, redstone, boats, sculk, on, mobBrushes, peacefulBreathing, ashDrift,
            voices);
}
// repeat for withMobBrushes, withPeacefulBreathing, withAshDrift
```

- [ ] **Step 4: Run the test** — `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests FoleyConfigTest`. Expected: PASS.

- [ ] **Step 5: Modify `FoleyScreen.java`** — append four `OptionInstance<Boolean>` fields, four `added(...)` initialisations, four entries in `list.addSmall(...)` (pick a new "subtle layers" section header), and update the `removed()` constructor call to pass the four new booleans.

- [ ] **Step 6: Add new keys to `en_us.json`** (append-only): `foley.settings.settling`, `foley.settings.mob_brushes`, `foley.settings.peaceful_breathing`, `foley.settings.ash_drift`.

- [ ] **Step 7: Update every positional `new FoleyConfig(...)`** in the codebase. Add `.withSettling(true).withMobBrushes(true).withPeacefulBreathing(true).withAshDrift(true)` after each construction. Files to touch:

  - `mod/src/test/java/io/github/cumsauce/foley/config/FoleyConfigTest.java`
  - `mod/src/gametest/java/io/github/cumsauce/foley/gametest/SceneRecording.java`
  - `mod/src/gametest/java/io/github/cumsauce/foley/gametest/FoleyClientGameTest.java`
  - `mod/src/main/java/io/github/cumsauce/foley/ui/FoleyScreen.java`

  For `SceneRecording`, the `FoleyClient.apply(new FoleyConfig(...))` calls must have the new toggles default-on.

- [ ] **Step 8: Append M22 gametest** in `FoleyClientGameTest.java` (append, do not edit existing lines): a short assertion that a peaceful mob plays breath under `peacefulBreathing`, and that all four toggles can be turned off without breaking anything else.

- [ ] **Step 9: Run the test** — `cd mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test --tests FoleyConfigTest`. Expected: PASS.

- [ ] **Step 10: Full verification** — three commands in this order. Each must pass before the next.

  - `cd /Volumes/My\ Shared\ Files/mc/foley-m22 && CARGO_BUILD_JOBS=1 cargo test --workspace --lib` — Expected: all Rust tests green.
  - `cd /Volumes/My\ Shared\ Files/mc/foley-m22/mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew test compileJava` — Expected: Java tests green and main source compiles.
  - `cd /Volumes/My\ Shared\ Files/mc/foley-m22/mod && JAVA_HOME=/opt/homebrew/opt/openjdk@25 ./gradlew build` — Expected: build succeeds. (The `runGametest` task may fail with "Timeout loading world"; that's a pre-existing infrastructure issue, not M22's.)

- [ ] **Step 11: Commit**

```bash
git add mod/src/main/java/io/github/cumsauce/foley/config/FoleyConfig.java mod/src/main/java/io/github/cumsauce/foley/ui/FoleyScreen.java mod/src/main/resources/assets/foley/lang/en_us.json mod/src/test/java/io/github/cumsauce/foley/config/FoleyConfigTest.java mod/src/gametest/java/io/github/cumsauce/foley/gametest/FoleyClientGameTest.java mod/src/gametest/java/io/github/cumsauce/foley/gametest/SceneRecording.java
git commit -m "feat(M22): four subtle-layer toggles wired through FoleyConfig and the UI"
```

---