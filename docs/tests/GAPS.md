# Sim coverage record

All the gaps found on 2026-10-15 are covered; this is the record of what was
added and what was measured, kept for whoever extends the sim next. The suite
(`cargo run --release -p foley-sim -- all`) runs in CI and gates every push.

## What the sim now covers beyond the original scenes

- **Cold starts** — `cold-wind/water/fire/lava/rain/thunder`: a bed created at
  a loud scene, which is how the game always makes one.
- **Roofs and foliage** — `roof-heights` (0.5–16 blocks), `wood-in-rain`
  (canopy overhead, leaves underfoot), `roof-stuff` (leaf, stone, metal, soft
  roofs), `downpour` (the game's loudest rain, then a low roof).
- **Thunder as flown** — `thunder-air` (the game's own temperature and
  humidity, swept mid-roll), `thunder-doorway` (shelter flipped while a roll
  plays), `thunder-near` (a bolt 50 m off; the guard must keep it from
  clipping, not from cracking).
- **Wind at flight and storm speeds** — `flight`, 8→50 m/s and down into a
  wood.
- **Water at river scale** — `bank-walk`, flow 20→300 past the bubble cap.
- **Fire and lava at common sizes** — `one-campfire` (0.6–3.6 blocks),
  `lava-shore` (area 1→200, gain 1→0.6).
- **Rain's ground** — `rain-grounds` (lake, stone, glass, metal), `drizzle`
  (the quietest rain).
- **Standing weather** — `long-weather`, ten minutes of rain, a campfire and
  lava.
- **Stop seams** — `stops`: each bed played, freed, and started again; the
  join must not step.
- **The concurrent mix** — `mix`: the storm's beds on one clock; the sum must
  keep headroom (measured peak −5.5 dBFS, −26.0 LUFS).

## Checks

Wind and water are checked for clicks, dropouts, crackle and read-steps as
before. Rain, fire, lava and thunder carry an `Expect` naming the fault their
scene is about: `NO_STEP` (dropouts + read-steps) or `NO_JUMP` (read-steps
only — their events are sparse and loud by nature, and thunder's flagged
"clicks" measured as 1–13 % of the roll's level in the gaps between segments:
dust under hearing, seed-dependent).

Each strike a track names carries its own position; thunder's arrival time is
the waveform's, not the harness's.

## Found and fixed along the way

- A wind bed restarted (cave left, setting switched on) dipped ~9 dB for
  ~100 ms: the leaves' envelopes and band-passes only charge by rendering.
  Fixed in `foley-dsp/src/wind.rs` — a fresh bed warms up over 8192 discarded
  samples before its first sample is heard.
- `play_with` hardcoded every strike at 900 m; tracks now carry positions.

## Deliberately not covered

- A full nine-stream mix with `SurroundBeds`' four decorrelated copies of each
  bed (pins the Java weighting, not the synthesis).
- Values the game cannot send (fuzz and the FFI guards cover the ranges).
