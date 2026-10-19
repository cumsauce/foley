# Changelog

The mod and the Rust workspace share one version; entries are grouped by that
version. Playtest builds are `0.1.0-beta.N`.

## 0.1.0-beta.63 — sample-rate groundwork

### Changed

- The synthesis crates now take the sample rate as a parameter throughout
  (beds, one-shots, voices, birds, instruments). The mod still renders at
  48 kHz and its audio is bit-identical to beta.62; this is groundwork for
  following the host's rate.
- Bed warm-ups and the cave's rumble filters scale with the sample rate.

### Fixed

- The water bed frees finished bubbles as it plays rather than once per render
  call, so a long render no longer runs out of voices and drops bubbles (the
  mod renders in short blocks and was not affected).
- Default bubbles and struck surfaces have a valid time step.

## 0.1.0-beta.62 — footsteps, blocks and explosions take over

### Added

- Foley now replaces the game's block **footsteps** with its own, and takes over
  them through the generic override registry like every other family. Once on,
  steps no longer play twice; `footsteps` in Options → Music & Sounds → Foley
  still gives them back.
- Block **break, place and digging** are Foley's too, under a new `blocks`
  setting (default on). It starts narrow — stone, wood, gravel, sand and glass —
  built from the existing contact synthesis; every other material still layers
  under the game's sound.
- **Explosions** booms are Foley's own: a new bang — a gliding sub boom, the
  shock front's broadband crack and seeded debris — whose blast radius sets its
  pitch, length and tail (at most three seconds). It plays through
  `FOLEY_SHOT_BOOM`, ABI rises to 25.

### Changed

- Note block strikes now go through the override registry, so an addon override
  wins over the `noteBlocks` setting exactly as it does for every other family.
- Contact layers (break/place/dig) now sound when either `strikes` or `blocks`
  is on, so taking a family over works with the additive layer switched off.

## 0.1.0-beta.61 — note block instruments (M23)

### Added

- Foley now plays its own note block strikes: sixteen instruments across five
  modelled families (plucked string, struck modal bars and bells, membrane
  drums, breath tubes, and the electronic bit and pling), each at any of the
  block's twenty-five notes and eight timbral variants. Nothing is resampled.
- `FOLEY_PRIMITIVE_NOTE` (`note(instrument, note, character, seed)`) renders a
  strike on demand; ABI rises to 24.
- `noteBlocks` in Options → Music & Sounds → Foley (default on) lets the
  vanilla note block plays through instead.

### Fixed

- The note block's `character` knob now reaches its hat as well.

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
