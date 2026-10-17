# Changelog

The mod and the Rust workspace share one version; entries are grouped by that
version. Playtest builds are `0.1.0-beta.N`.

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
