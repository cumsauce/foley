# Doorswing × Foley

A pasteable integration guide for [doorswing] — a client-side mod that animates vanilla doors and trapdoors — to replace the game's flat open/close clicks with synthesized creak, scrape, squeak and clunk layers scaled to the animation's velocity, through Foley's sound-override API.

Everything below is client-side. A server never sees it, and a client without Foley simply never reports `Foley.available()`; there is nothing to handle there. Outside the `foley` entrypoint, check `Foley.available()` first.

## 1. The dependency

`foley-api` is published to Maven repositories. In a Loom build:

```gradle
repositories {
    mavenLocal()
}

dependencies {
    compileOnly "io.github.cumsauce:foley-api:0.1.0-beta.59"
}
```

Use **`compileOnly`** — do **not** use `include` / jar-in-jar. Foley already bundles the api jar at runtime; doorswing must not ship a second copy.

To regenerate the api locally (for example while iterating against an unreleased Foley):

```bash
cd mod && JAVA_HOME=<jdk25> ./gradlew :api:publishToMavenLocal --no-configuration-cache
```

## 2. The generic override API

Foley hands you vanilla sound events before the game plays them, for any event you can name — not just doors:

```java
SoundOverrideRegistration reg = api.soundOverrides()
        .register(match, handler);   // Predicate<Identifier>, VanillaSoundOverride
reg.unregister();                    // drop just this override
api.soundOverrides().clear();        // drop every override, sugar registrations included
```

The handler:

```java
@FunctionalInterface
public interface VanillaSoundOverride {
    /** @return true to keep the vanilla event skipped; false to let it play */
    boolean onVanillaSound(Identifier vanillaEvent, double x, double y, double z);
}
```

Semantics, precisely:

- An override is consulted **after** Foley's global bypass check. While Foley is bypassed, the vanilla sound plays and your handler is **not** called.
- Overrides are consulted in registration order; the **last-registered override whose predicate matches** decides the outcome for that event.
- Returning **true** keeps the vanilla event skipped — you have taken it over. Returning **false** lets the game's sound play as the fallback.
- The callback itself **does not** spend Foley's per-tick one-shot budget. The sounds you play via `sounds().play(...)` from inside it do, precisely so a takeover cannot crowd out the game's own sounds.
- Registration is settable any time on the client thread (from your entrypoint, from a keybind, from a config reload — wherever).

## 3. Doors specifically (back-compat sugar)

Doors are the common case, so `doorHandler` sits on top of the same registry:

```java
api.doorHandler((event, x, y, z, iron) -> {
    // play something
    return true;
});
api.doorHandler(null);   // restore Foley's built-in contact tap
```

- Door and trapdoor events **include iron** doors and trapdoors; the `iron` flag tells you which.
- Open vs close is derivable from the event id (it ends in `.open` or `.close`).
- The handler fires **even when Foley's `doors` toggle is off** — that toggle only governs Foley's own built-in contact tap, not whether your handler is consulted.
- Like the generic registry, the handler is skipped while Foley is bypassed (the vanilla sound plays then).

## 4. A doorswing-flavoured example

Register in your `FoleyEntrypoint` (or any time on the client thread). Match the eight door events, work out material and action from the id, and play a velocity-scaled creak — doorswing's animation velocity drives gain and pitch, so a slow heavy swing groans and a fast one snaps:

```java
public final class DoorswingFoley implements FoleyEntrypoint {
    private static final Set<Identifier> DOOR_EVENTS = Set.of(
            Identifier.of("minecraft", "block.wooden_door.open"),
            Identifier.of("minecraft", "block.wooden_door.close"),
            Identifier.of("minecraft", "block.iron_door.open"),
            Identifier.of("minecraft", "block.iron_door.close"),
            Identifier.of("minecraft", "block.wooden_trapdoor.open"),
            Identifier.of("minecraft", "block.wooden_trapdoor.close"),
            Identifier.of("minecraft", "block.iron_trapdoor.open"),
            Identifier.of("minecraft", "block.iron_trapdoor.close"));

    @Override
    public void onFoleyReady(FoleyApi api) {
        api.soundOverrides().register(DOOR_EVENTS::contains, (event, x, y, z) -> {
            String path = event.getPath();                       // e.g. block.iron_door.close
            boolean iron = path.contains("iron");
            boolean opening = path.endsWith(".open");

            // doorswing: the swing's current angular velocity, normalized to 0..1
            double v = doorswing.velocityAt(x, y, z);            // your own animation state
            float gain = (float) (0.25 + 0.75 * v);              // louder the faster it swings
            float pitch = (float) (iron ? (0.7 + 0.3 * v)        // iron: low, heavy
                                        : (1.0 + 0.5 * v));      // wood: lighter, tighter

            boolean creak = api.sounds().play(
                    Identifier.fromNamespaceAndPath("doorswing", iron ? "creak_iron" : "creak_wood"),
                    x, y, z, SoundSource.BLOCKS, gain, pitch);
            if (!opening) {
                api.sounds().play(
                        Identifier.fromNamespaceAndPath("doorswing", iron ? "clunk_iron" : "clunk_wood"),
                        x, y, z, SoundSource.BLOCKS, gain * 0.8f, pitch * 0.9f);
            }
            return true;   // vanilla event stays skipped
        });
    }
}
```

The models behind `doorswing:creak_wood` and friends can be registered in the same entrypoint (`api.sounds().register(id, model, variants)`) or declared in a resource pack under `assets/doorswing/foley/sounds.json` — see `docs/INTEGRATING.md` for both paths and the full model catalogue.

`play` returns whether it actually played (false while Foley is bypassed, at zero volume, or past its share of one-shots for the tick). Ignoring the return is fine — but note the budget is spent either way, so keep per-event layers to one or two.

`api.doorHandler(...)` does the same job for this case if you prefer the sugar; the generic form above is only needed when you want per-event predicates, multiple competing registrations, or `unregister()` lifecycles (e.g. tear down on config reload).

## 5. Event ids, for reference

| Event id |
|---|
| `minecraft:block.wooden_door.open` |
| `minecraft:block.wooden_door.close` |
| `minecraft:block.iron_door.open` |
| `minecraft:block.iron_door.close` |
| `minecraft:block.wooden_trapdoor.open` |
| `minecraft:block.wooden_trapdoor.close` |
| `minecraft:block.iron_trapdoor.open` |
| `minecraft:block.iron_trapdoor.close` |

Modded doors are **not** in this list by default — match them with a predicate of your own (a namespace check, a block lookup) if doorswing should cover them too.
