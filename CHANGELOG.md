# Changelog

The mod and the Rust workspace share one version; entries are grouped by that
version. Playtest builds are `0.1.0-beta.N`.

## 0.1.0-beta.60 — generic sound overrides

### Added

- `foley-api` gains `SoundOverrideRegistry`, reachable as
  `FoleyApi.soundOverrides()`: any addon can take over any vanilla sound Foley
  already handles, not just doors. Overrides are keyed by an `Identifier`
  predicate, so one registration can cover a family of events.
- The last registered override that matches wins — returning true keeps the
  vanilla event skipped, false lets it play. The registration handle
  `unregister()`s itself, and `clear()` drops every override at once.
- The global bypass still wins: when it's on, the override isn't consulted and
  vanilla plays.
- `DoorSoundHandler`/`doorHandler` stay as sugar for the door case; existing
  door code keeps working unchanged.
- Mob voice sounds are outside this registry: they run through Foley's separate
  voice system.

### Fixed

- Foley's global bypass now lets the game's own sounds through for every event
  Foley takes over (rain, thunder, ambience, drips, chests, shulkers,
  minecarts), not just doors.

## 0.1.0-beta.59 — pluggable door sounds

### Added

- `foley-api` gains `DoorSoundHandler`: an addon can register a handler on
  `FoleyApi` to take over the vanilla door/trapdoor open/close sounds from
  Foley's built-in contact tap. Returning true keeps the vanilla event skipped,
  false lets it play; no handler means today's behaviour exactly. Registrable
  any time on the client thread; `null` restores the built-in tap. Documented
  with an example in `docs/INTEGRATING.md`.
- `foley-api` is now publishable to Maven repositories (`publishToMavenLocal`
  installs the api jar, sources jar and pom under `io.github.cumsauce:foley-api`),
  so sibling mods can compile against it.

## 0.1.0-beta.58 — coastline fixes

### Fixed

- Wind leaves' rustle now sits inside its level budget, so a gale no longer
  lifts the leaf layer above the wind bed's ceiling.
- Surf scene sanitisation: non-finite heights are treated as silence instead of
  poisoning the scene, the grain mix is returned to the caller, and the storm
  cadence is re-armed after a reset instead of going quiet.
- A fresh bed's guard now starts at the ceiling, not the floor: no fade-in and
  no onset overshoot on the first buffer.
- Native render length is bounded by the destination buffer's capacity, and a
  negative reported length is refused rather than trusted.
- Guarded the `BufferSound`, `GeneratedSounds` and `AddonSounds` accessors,
  including variant indices, so a missing or out-of-range sound cannot panic
  the audio thread.
- A bed closes its previous stream when it changes, and the underwater latch
  now flips on the 20th dry tick (the off-by-one that left it stuck on).
- Removed the dead `surf_0`/`surf_1`/`surf_2` one-shot events from
  `sounds.json`.

### Added

- `cargo xtask levels` measures bed output against the 0.4 ceiling, so level
  regressions are a command rather than an ear.
- Opt-in `audio-debug` logging of bed levels (`-Pfoley.audioDebug=true`), for
  capturing a playtest log without a debugger attached.
