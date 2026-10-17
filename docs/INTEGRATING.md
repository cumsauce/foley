# Integrating with Foley

Foley's synthesis, for other mods: define a sound from one of its models — in code or in a resource pack — and play it in the world. No sound files, no sounds.json of your own; the variants are synthesized the first time each is heard and cached like any sound. Everything runs through the game's own channels, so Earshot shapes addon sounds exactly as it shapes Foley's, and they follow Foley's volume and its bypass key.

All of it is **client-side**. Servers never see it, and a client without Foley simply never reports `Foley.available()`; there is nothing to handle.

## The dependency

```gradle
// TODO: published to the Modrinth maven; until then, build foley-api from source (mod/api) and jar-in-jar it.
repositories { maven { url = 'https://api.modrinth.com/maven' } }
dependencies { modImplementation 'maven.modrinth:foley-api:<version>' }
```

While Foley is in beta, foley-api is versioned in lockstep with it (no separate releases). Until you publish against the maven, you can depend on the project or bundle the jar; what must not happen is a *second* copy of Foley itself.

Your mod's `fabric.mod.json`:

```json
"depends": { "foley": "*" },
"entrypoints": { "foley": [ "com.example.mymod.MyFoley" ] }
```

The entrypoint is ignored when Foley is absent — integrations are soft by construction. If you call the API outside it, check `Foley.available()` first.

## In code

```java
public final class MyFoley implements FoleyEntrypoint {
    @Override
    public void onFoleyReady(FoleyApi api) {
        api.sounds().register(Identifier.fromNamespaceAndPath("mymod", "big_drip"),
                new SoundModel.DripIntoWater(2.5f), 4);
    }
}
```

…anywhere on the client thread:

```java
if (Foley.available()) {
    Foley.api().sounds().play(id, x, y, z, SoundSource.AMBIENT, 1f, 1f);
}
```

- **Register** only within the entrypoint; it closes when the last addon returns. The id's namespace must be your mod's. Each of the variants (up to 64) is its own rendering, picked among at random.
- **Play** returns whether it played: false while Foley is bypassed, at zero volume, or past its share of one-shots for the tick. Addon sounds share Foley's budget precisely so they cannot crowd out the game's own sounds.
- The models are the whole current catalogue — see `SoundModel`. Each checks its parameters when built, so mistakes fail where you make them. `Contact` and `Voice` give you Foley's block physics and mob voices under your control.

## In a resource pack

`assets/mymod/foley/sounds.json`:

```json
{
    "big_drip": { "model": "drip_into_water", "radius_mm": 3.0 },
    "night_howl": { "model": "howl", "variants": 2 }
}
```

Same fields as the records, plus `variants`. A bad entry is logged (with file and name) and skipped; it never breaks the reload. Pack sounds are re-read on every resource reload. A code definition wins over a pack's for the same id.

Subtitles and the game's own variant picking need a sounds.json entry; if you want them, declare the variant paths there (`"sounds": ["foley/big_drip_0", …]` in your namespace) — Foley keeps them through the game's file check and serves their buffers. Without it, the plain `play` path needs nothing but this API.

## Your own materials (F1)

Foley's strikes, breaks and footsteps know ten materials. Yours can have their own physics: describe the block with a `MaterialDescriptor`, map blocks onto it, and Foley layers those blocks' sounds with your constants — glassy ones ring bright and shatter into shards, stiff heavy ones clang.

```java
public final class MyFoley implements FoleyEntrypoint {
    @Override
    public void onFoleyReady(FoleyApi api) {
        Identifier crystal = Identifier.fromNamespaceAndPath("mymod", "crystal");
        api.materials().register(crystal, new MaterialDescriptor(
                SoundModel.MaterialKind.GLASS,  // whose habits it keeps: how it grains, scuffs, crunches, cracks
                90e9, 3100, 0.2, 0.01,          // Young's modulus (Pa), density, Poisson, loss
                0.05,                           // the vibrating plate's thickness (m)
                24, 0.5, 0.003));               // fragments it shatters into, restitution, shard thickness
        api.materials().mapSoundType(SoundType.AMETHYST, crystal);   // all its sounds
        api.materials().mapBlock(Blocks.AMETHYST_BLOCK, crystal);    // or the exact block
        api.materials().mapTag(BlockTags.CRYSTAL_LIKE, crystal);     // or a tag
    }
}
```

The descriptors ship with shortcuts (`MaterialDescriptor.glassy()`, `metallic()`, `stony()`, `wooden()`, `soft()`), and each value is checked where you build it. Blocks are looked up by exact block, then a tag, then their sound type, then Foley's ten — a mod's own `SoundType` is the natural key, since it covers a break, whose block is already gone when Foley layers it. Custom layers follow the same "Strikes and breaks" and "Footsteps" toggles as Foley's own.

In a resource pack, `assets/mymod/foley/materials.json`:

```json
{
    "materials": {
        "crystal": { "behavior": "glass", "young_gpa": 90, "density": 3100, "poisson": 0.2,
                      "loss": 0.01, "thickness": 0.05 }
    },
    "blocks": { "mymod:crystal_block": "crystal" },
    "tags": { "c:crystal_like": "crystal" }
}
```

## Primitives: sounds of your own (F2)

Beyond Foley's catalogue, four parameterized primitives compose sounds of your own invention — a bubble's ring, a burst of shaped noise, a gliding tone, a gust — each rendered bounded and peak-normalized, whatever you feed them:

```java
api.sounds().register(Identifier.fromNamespaceAndPath("mymod", "chime"),
        new SoundModel.Tone(440f, 660f, 0.3f, 0.5f), 4);
```

In a pack, the same names: `{ "model": "noise", "centre_hz": 2000, "q": 1.2, "seconds": 0.3, "gain": 0.8 }`. A registerable sound is one model; to layer, register several under different names and play them together, or lean on the game's own mixer. (Render-level mixing, several models under one name, may come later.)

## Beds (F2)

For a sound that runs while your mechanic does — a machine, a portal, a river you placed — Foley's streaming models can play under your id, told their scenes as your world changes:

```java
FoleyBeds beds = Foley.api().beds();
if (beds.start(id, FoleyBeds.BedKind.WATER, new float[] {20f, 0f, 1f})) {   // flowing, no falls, full gain
    beds.place(id, x, y, z);        // play from where your thing is
    beds.scene(id, new float[] {60f, 0f, 1f});  // as it changes; the sound glides there
    beds.weight(id, 0.5f);          // half of Foley's volume
    beds.stop(id);
}
```

`start` returns false past the budget: the game allows 8 streaming sounds at once, the game's music and Foley's own beds take most, so **at most 2 beds may addons run at once**. Beds come and go whenever your mod likes (client thread), and each kind's scene is documented on `FoleyBeds`.

## Creature voices (F3)

Foley can speak for your modded creatures: describe the voice — a vocal tract's length and a pitch (which together read as the body's size), plus how far the creature goes into roughness and breath — and Foley makes its ambient, hurt, death, growl and breath calls from the physics, in the game's own sound events, with the game's volume and pitch kept and its subtitles kept too.

```java
api.voices().register(myCreatureType, new VoiceDescriptor(0.35, 90, 0.7, 0.4),
        Map.of("ambient", "ambient", "hurt", "hurt", "death", "death",
               "growl", "growl", "snort", "breath"));
```

The map takes the game's call names (as your entity's sounds name them) onto the five archetypes; a call you don't map — nor its name without a `_baby` suffix — keeps the game's own sound. Variant suffixes (`_baby`, `_big`, …) pitch the voice as the game's do. Players gate all of it under "Addons' Creatures" in Foley's settings, off by default like every voice group.

In a pack, `assets/mymod/foley/voices.json`:

```json
{
    "mymod:deep_one": {
        "voice": { "tract_length_m": 0.5, "pitch_hz": 70, "rough": 0.8, "breathy": 0.3 },
        "calls": { "ambient": "ambient", "hurt": "hurt", "death": "death" }
    }
}
```

## Sound overrides (S1)

A mod can take over any vanilla sound Foley itself takes over. Register an override — in your `onFoleyReady`, or any time on the client thread — keyed by a predicate on the event id, and Foley stops playing its own sound for whatever matches: each vanilla event is handed to you with its id and its world position. Return true when you played something, keeping the vanilla sound skipped; return false and the game's sound plays as the fallback.

```java
// Override any vanilla sound Foley takes over, keyed by event id:
api.soundOverrides().register(id -> id.getPath().startsWith("block.wooden_door."),
        (event, x, y, z) -> {
            api.sounds().play(Identifier.fromNamespaceAndPath("mymod", "door_creak"), x, y, z,
                    SoundSource.BLOCKS, 0.6f, 1f);
            return true;
        });
```

Doors remain the common case, so `api.doorHandler(...)` and `DoorSoundHandler` stay as sugar over the same registry: they match the door and trapdoor events, tell you whether the event was iron, and Foley's built-in contact tap stops while a handler is set.

```java
api.doorHandler((event, x, y, z, iron) -> {
    api.sounds().play(Identifier.fromNamespaceAndPath("mymod", iron ? "door_iron" : "door_wood"),
            x, y, z, SoundSource.BLOCKS, 0.6f, 1f);
    return true;
});
```

Open and close are told apart by the event id (`minecraft:block.wooden_door.open`, `minecraft:block.iron_trapdoor.close`, …). Foley's bypass wins: while bypassed the game's sound plays and no override is consulted. Otherwise the last-registered override whose predicate matches decides; `register` returns a handle whose `unregister()` removes that one override, and `api.soundOverrides().clear()` removes them all — the door handler's sugar registration among them. Whatever you play through `sounds().play` follows Foley's volume, bypass and per-tick one-shot budget; the callback itself does not spend that budget. `api.doorHandler(null)` restores Foley's built-in tap. Mob voice sounds are not part of this registry: they run through Foley's separate voice system.

## What is deliberate

- The `foley/` path within a namespace is Foley's, by convention: variant buffers live at `<ns>:foley/<name>_<n>`.
- One addon's failure is logged with its mod id and skipped; it never takes Foley or the game down.
- Rendering happens on the game's background executor, as file loads do, and buffers are cached until a resource reload.
