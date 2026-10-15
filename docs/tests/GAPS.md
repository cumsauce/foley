# Scenarios the sim doesn't yet cover, ranked

Written 2026-10-15 from a read of `crates/foley-sim`, the controllers that feed the
beds (`RainScene`, `WindScene`, `ThunderScene`, `Surroundings`, `SurroundBeds`,
`PlacedBeds`, `BedSound`, `Layers`), the FFI (`foley-ffi/src/lib.rs`) and the synth
(`foley-dsp`, `foley-synth`). Nothing here is a claim that the code is wrong: each
entry is a scenario the game produces that no scene, walk or fuzz exercises.

> **Status (2026-10-15, final — everything authored, reviewed, green, pushed).** The scenes, the
> per-track expectations and the stop-seam check are in `crates/foley-sim`; see
> "What was added" at the end for exactly which. The suite (`foley-sim all`)
> passes. The one real defect the tests found — a wind bed stopped and started
> again dipped ~9 dB at the join — is fixed in `crates/foley-dsp/src/wind.rs`
> (a fresh bed now warms its render up before its first sample is heard). Two
> tests were also corrected on review: the harness had hardcoded every
> lightning strike at 900 m, so the "close bolt" scene was lying (tracks now
> carry each strike's position, and `thunder-near` really strikes 50 m off),
> and thunder's expectation is steps-only — the click detector fires on
> numerical dust in the gaps between roll segments (measured at 1–13 % of the
> roll's level, far under hearing, seed-dependent), so a clicks check would
> have been flaky.

Read the priorities below against what `scenes.rs` already has:

- `forest-walk` wind 1.5→6 m/s, exposure 1→0.5→0.9, leaves 0→0.9
- `storm` rain 0→60 with a roof from t=50, wind 3→15 then indoors, thunder fixed
- `waterfall-pass` water flow 5→30, falls 0→12, gain 0.15→1
- `campfire` fire 1→25, lava area 60 gain 0→0.4
- `long-woods` wind ±2 about 4 m/s, water flow 20 falls 2 gain ±0.3, ten minutes
- `snaps` wind/water/rain jumping between corners every 5 s
- `walks` wind 0–20 m/s, water flow 0–60 / falls 0–15 / gain 0–1, 3 seeds × 10 min
- `fuzz` every bed, params anywhere, every tick, reads 1 sample–1 s, 60 s

`play_with` models the game's handoff (a pending scene taken up at read boundaries,
0.5 s reads). Fuzz is the only thing that varies read size and it is not "smooth", so
dropouts/clicks/crackle/read-steps are checked **only** for wind and water — rain,
fire, lava and thunder fail only on clipping. Several gaps below are about a step or
a dropout in a bed that will therefore pass with a nonzero count; see "Notes for the
test writer" at the end.

---

## P0 — cold starts and stop seams  (the highest-risk gap)

**Every bed in the game is created because the world just got loud, and the sim
never does this.** Every scene calls `play_with` with a bed whose first scene is
already the smooth `params(0)`; nothing creates a bed at a strong scene. In game:

- `ThunderController.strike` **creates the bed and strikes it in the same call**
  (`BedSound` is made, played, then `sound.strike(...)`). Thunder's first render is
  a fresh synth meeting a flash.
- A fire lit, a lava pocket mined open, a waterfall walked up to: `PlacedBeds`
  starts a bed at (fire 1, gain 1) / (lava 1, gain 1) / (water 0–10, falls 20–50,
  gain 1) on the very first sample it is heard.
- Wind starts at whatever the first sample gives (a rooftop in a storm: 15–25 m/s,
  exposure 1) and rain starts as the controller's rate eases up from 0 — but a
  `/foley rain 100` forces a hard rate at once.

The synth's cold-start behaviour is unverified: `Wind::set_scene` on a `fresh` bed
sets the *heard* speed to the target immediately (the gust history starts empty),
and Fire/Lava/Water start from zero filter state. This is exactly the case the
`BED_CEILING` guard comment names ("a scene that jumps (a waterfall appearing, a
gust) is caught"). Scenes to add (scenes-only):

- `cold-wind`: wind `|_| vec![20.0, 1.0, 0.9]`, 30 s.
- `cold-water`: water `|_| vec![0.0, 40.0, 1.0]`, 20 s (a fall appearing at the ear).
- `cold-fire`: fire `|_| vec![5.0, 1.0]`, 20 s.
- `cold-lava`: lava `|_| vec![4.0, 1.0]`, 20 s.
- `cold-rain`: rain `|_| rain(40.0, 0.5, 0.2, None)`, 20 s.
- `cold-thunder`: thunder with `strikes: &[0.5]` — a flash on a just-created bed.

**The stop side is worse, because one bed has no fade at all.** The sim never
destroys a bed mid-play, so no seam is ever measured. In game:

- Wind: `WindController.tick` calls `SurroundBeds.stop` when speed < 0.5 m/s or
  (exposure < 0.02 and leaves < 0.05) — entering a cave, diving, the Nether portal.
  `SurroundBeds.stop` stops every bed dead, and `BedSound` has **no fade** on stop
  (water/fire/lava got a 3 s fade in commit `cab971e`; wind did not). This is a
  likely audible click in game and is worth a unit/gametest, not a sim scene:
  assert `weigh(0)` is asked for before `stop`, or that the bed ramps down. **Route
  this to a Java test, not the sim.**
- Rain/thunder stop on their linger timers (`LINGER` 100 ticks / 25 s), by which
  time they are quiet.
- The native seam can be tested: render bed A, `foley_bed_destroy`, create bed B
  with the same scene, concatenate, measure the step at the join. Needs a small
  addition to the play loop (the one harness change in the scene-only groups).

---

## P0 — rain under roofs and in foliage

The roof is the dominant element for a sheltered listener (85 % of near drops drum
on it, `Rain::drop`), and the sim tests exactly two: `storm` runs roof wood at
height 3 from t=50, `snaps` glass at height 3 in one of four states. In game the
common values are:

- **Heights.** `RainScene.sample` stores `height = pos.getY() - eye.y`, so the
  in-game minimum is about 0.4 (a 2-high space: the first solid block is the ceiling
  at eyeY+1). The drumming gain is `1.5 / height.max(1.0)`, so a 3-high room (1.4)
  and a doorway crawl (0.4→1.0) are ~4.5 dB louder than the sim's height 4. Tall
  atriums reach `ROOF_REACH` = 16. A scene sweeping height 0.5 → 16 (roof on) puts
  the law at both ends.
- **Surfaces.** Only wood and glass are ever tried. Missing, all reachable via
  `Surfaces.of`: **leaves** (standing under any tree — the default outdoor shelter),
  **stone** (cave mouth, overhang), **metal** (copper roof, iron blocks, anvil),
  **soft** (wool ceiling). Each picks different modes and loudness in `Struck::new`.
- **Rain in a wood**, the most common way rain is heard outdoors: ground shares go
  to `leaves ≈ 1.0` (every column's top block is canopy). The sim only ever uses
  leaves = 0.3. A forest scene (rate 10–50, leaves ground ~1.0, roof leaves at 4–6)
  is missing entirely.
- **The loudest legal steady rain.** The game's max is `10 + 40` = 50 mm/h; `storm`
  ramps through it but never sits on it, and never on it *under a low roof*.

Scenes: `roof-heights` (rain 10, wood roof, height sweeping 0.5→16, 60 s);
`wood-in-rain` (rate 15, leaves 0.95, wood 0.05, roof leaves 5, 60 s); `downpour`
(rate 50 steady, then roof wood height 2 from t=30, 60 s).

Two encoding traps in `scenes.rs`'s helpers:

- `rain(rate, soft, leaves, roof)` takes the roof as `Option<(f32, f32)>` where the
  tuple is **`(surface index, height)`**, written to `p[1 + FOLEY_SURFACES]` and
  `p[2 + FOLEY_SURFACES]`. A leaf roof is `Some((2.0, 5.0))`, a metal roof
  `Some((6.0, 2.0))` — not `(height, surface)`. Getting it backwards silently yields
  an unknown-surface error (`apply` rejects it), so the test fails loudly, not quietly.
- `rain_on_water` hardcodes water 0.5 + soft 0.5 and takes no leaves; a pure-forest
  or pure-glass scene must call `rain()` directly. Surface indices: water 0, stone 1,
  leaves 2, wood 3, soft 4, glass 5, metal 6.

---

## P1 — thunder's normal case

The sim's thunder is always created at 0, told a scene of 18 °C / 85 % — a pair the
game **never sends** (`ThunderScene.sample` gives 95 % while raining, 60 % otherwise,
and celsius 0–40 from biome temperature) — and struck at exactly 900 m, y = 0, with
`seed = index`, every 13–17 s. Missing:

- **Cold first strike** (P0, repeated here because it is *every* bolt, not an edge).
- **The game's air.** celsius and humidity change the far-roll absorption
  (`absorption_db_per_m`, `strike`): a scene sweeping celsius 0→40 and humidity
  95→60 across a long roll pins it. Note the split: celsius/humidity are **baked at
  strike time** into the fabricated waveform, while exposure is applied **live at
  render** (`Thunder::render`: muffle + `0.4 + 0.6 * exposure` gain). A storm
  starting or ending mid-roll flips humidity 95↔60 while a roll is in flight.
- **Scene flips while thunder is in flight.** At 900 m a strike arrives 2.6 s later;
  running indoors or rain starting mid-flight is ordinary. This is the code that was
  patched for a stale-filter click ("bypassed in the open and taken up again at a
  doorway… a click mid-rumble"), and a test should pin it: strike, then flip
  exposure 1→0→1 while the roll plays.
- **Distance.** 900 m only, with checks. The gametest's bolt is 40 m. A 20–60 m crack
  exercises the guard's first transient. (`thunder-near` covers this now. The sleep
  is no caveat after all: the arrival delay is baked into the fabricated waveform —
  `thunder.rs`'s own test asserts first-heard at 0.87 s for 300 m — and the 300 ms
  sleep only gives the worker thread time to finish before the next render takes
  the sound up. Timing is the waveform's, not the harness's.)
- Two strikes a few seconds apart (overlap and guard ducking). Vanilla spacing is
  10–30 s, so keep this to one line; do not build a busy-storm marathon.

---

## P1 — wind at flight and storm speeds (25–60 m/s)

In-game wind reaches: clear 5, rain 9, storm 16 × log-profile (0.1–2.5) plus the
player's own motion past 8 m/s, capped 60. Flying/gliding puts it at 25–60. The
sim's scenes top out at 25 (`snaps`) and the walks at 20 (and their distribution is
`squared`, so mostly under 10). Fuzz throws 60 but only clip-checks, and gust σ
grows with speed, so the loudest regime is also the gustiest — exactly where
smoothness should be checked. One scene: an elytra ride (speed easing 8→50→8 over
~10 s, exposure 1, leaves 0), landing, then walking into a wood (leaves 0.9, hitting
the 1.2 m/s controller floor). **Do not** try to cover this by widening the walk's
wind range instead: the walk draws `lo + (hi - lo) * rng.unit().powi(2)`, so it is
squared-biased toward zero — at `(0.0, 60.0)` the median draw is still only ~5 m/s
and 50+ appears in roughly the top 1 % of keyframes. A dedicated scene is the only
way to actually spend time in the regime.

---

## P1 — water at river scale, and gain sweeps

The walk caps flow at 60. A river or lake edge puts 100s of flowing surface blocks
in the 24-block scan (32 with Earshot Ultra). At exactly those magnitudes the synth
changes regime: the bubble rate hits `MAX_BUBBLES` at flow 50 (`60 * flow`) and
starts boosting each bubble's level up to 1.5×; the rush saturates at flow 100;
the roar saturates at falls 12. The most common way water is heard — walking a bank
as gain swings 0.08→1 with d_eff and flow steps 20→300 — crosses the flow-50 kink
where the level law changes slope. This is the water analogue of the far-stream
dropout the walks were built to catch. Ranges tweak on existing machinery:
`walks` water flow `(0.0, 60.0)` → `(0.0, 400.0)`, and add a `bank-walk` scene
(flow 20→300, gain 0.08→1, falls 0→20).

---

## P1 — the concurrent mix — done

The game runs up to ~9 streams in an ordinary storm (4 rain + 4 wind + water/fire/
lava parts + thunder; ~21 in the worst case), and the sim never rendered two beds
at once, so the summed level was unmeasured while a single rain bed peaks at
−3.3 dBFS. `foley-sim mix` now plays the storm's three beds together on one clock,
each on its own scene schedule, and fails if any sample of the sum reaches full
scale. Measured: **peak −5.5 dBFS, −26.0 LUFS** — comfortable headroom. (This pins
the native levels only; `SurroundBeds`' "weights' squares sum to 1" argument keeps
the Java side as loud as one bed, and its four decorrelated copies of each
controller's bed are what the −5.5 dBFS must not be doubled by. A full nine-stream
mix with the four-fold copies is possible later, but the storm mix is the case a
clip would first come from.)

---

## P2 — fire and lava at their common small scales

- **Campfire**: size 0.6 per lit campfire (`Surroundings.sample`), the most common
  fire in game; the sim's fire starts at 1.0 and ramps only upward. Sizes 0.6–3
  (one campfire, two netherrack blocks) are absent, and fire that *spreads* steps
  +1 at a time rather than gliding. Scene: fire stepping 0.6→1.6→3.6→1.6, 45 s.
- **Lava**: area is always 60 in scenes, with gain ≤ 0.4. A single block (mining
  into a pocket, the most common lava encounter) is area 1 at gain 1. Note the burst
  rate `BURSTS * area` with `BURSTS = 0.35` caps at area ≈ 86, so 1 and 60 are both
  in the same (uncapped) regime — the difference is level, not texture. Missing:
  gain 1 (a big lake edge; the sim never exceeds 0.4), and area stepping by 10s of
  blocks as you walk a shore. Scene: lava `[(1, 1), (40, 1), (14, 1), (200, 0.6)]`
  held ~5 s each.

---

## P2 — long idle steady states

`long-woods` exists because "faults come with time", but only wind and water get one.
Rain/fire/lava/thunder never run past 70 s. Standing in steady rain (rate 10) or
beside an idle campfire or lava lake for 20 minutes is normal. A steady 10-minute
scene each (or extending the walk to cover them) catches slow drift — voice-pool
behaviour, guard wander, RNG correlations — the class the walks were built for.

---

## P2 — rain ground-share exotica

Cheap scene lines: glass and metal ground shares (the synth's glass/metal drops
otherwise appear only via the roof path); water share → 1.0 (sizzle at max plus the
384-bubble `MAX_VOICES` cap); a light drizzle (rate 0.5–2, soft 1.0) — the quietest
real rain, worth checking it is not near the dropout/level floors; and fast surface
stepping (0.5 s cadence) as when flying over a shoreline.

---

## Notes for the test writer

- **Non-smooth beds only fail on clipping.** A new rain/fire/lava/thunder scene
  passes even with clicks: the current `storm` rain already reports 20 clicks and
  `snaps` rain 8, and neither fails. If a scenario's whole point is a step or a
  click, either extend the check (the read-steps metric is already computed and
  printed for every bed) or assert the metrics/baseline explicitly. Thunder reports
  ~98 "dropouts" naturally — the gaps between segments of a far roll — and these
  are not failures.
- ~~The fuzz doc comment promises "no stretch may go silent while it plays"~~ — fixed
  to match what fuzz checks (no failed calls, no clipping).
- `play_with`'s 300 ms sleep per flash: not a timing fiction after all. The arrival
  delay lives in the fabricated waveform (thunder's own test asserts it); the sleep
  only gives the worker thread time to finish before the next render takes the
  sound up.
- Do **not** add scenes for values the game cannot send: rain rate 150 sustained,
  thunder humidity 85, roof height 64, and out-of-range clamps. Fuzz and the FFI
  guards already cover those.
- The gametest already checks the *plumbing* for all of this (rain starts and is
  replaced, the roof reads as wood, wind blows outdoors and drops under a roof,
  water/fire/lava beds start and are placed, a bolt plays, drips and layers fire).
  The gaps here are about *what the sound does*, which is the sim's job.
- ~~Scenes-only groups…~~ All authored now (see below). Harness additions that were
  needed: the stop-seam measurement (`stops`), strike positions on tracks, and the
  concurrent mix (`mix`). Still open, deliberately: a full nine-stream mix with
  `SurroundBeds`' four decorrelated copies of each bed.
- The unfaded wind stop on the Java side (`SurroundBeds.stop` stops dead, where
  `PlacedBeds` fades): resolved by analysis, not code — both stop conditions fire
  from a near-silent level already (speed < 0.5 m/s puts the wind bed ~48 dB under
  its 10 m/s level; exposure < 0.02 muffles it by ~10 dB first), so the cut lands
  on almost nothing to cut. Worth remembering if the thresholds ever loosen, but
  not a defect today.

---

## What was added (2026-10-15)

Scenes (`crates/foley-sim/src/scenes.rs`) — 20 new, one per gap above where a scene
could reach it:

| Scene | Covers |
| --- | --- |
| `cold-wind`, `cold-water`, `cold-fire`, `cold-lava`, `cold-rain`, `cold-thunder` | P0 cold starts: a bed created at a loud scene |
| `roof-heights`, `wood-in-rain`, `roof-stuff`, `downpour` | P0 roofs at 0.5–16 blocks, leaf/stone/metal/soft roofs, a leaf ground under a wood, the 50 mm/h downpour |
| `thunder-air`, `thunder-doorway`, `thunder-near` | P1 thunder: the game's air, a flip mid-roll, a 50 m bolt (tracks carry each strike's position; `play_with` used to hardcode 900 m) |
| `flight` | P1 wind at 8→50 m/s and the leaves' floor |
| `bank-walk` | P1 water at river scale (flow 20→300, past the bubble cap) |
| `one-campfire`, `lava-shore` | P2 fire 0.6–3.6, lava area 1–200 at gain 1→0.6 |
| `rain-grounds`, `drizzle` | P2 ground shares (lake, stone, glass, metal) and the quietest rain |
| `long-weather` | P2 ten minutes of rain, a campfire and lava |

Expectations (`scenes::Expect`, checked in `main.rs` via `analysis::expected`):
rain, fire, lava and thunder now name the faults their scenario is about, instead
of only failing on clipping. Three presets: `NO_STEP` (dropouts + read-steps, for rain, wind
and water scenarios) and `NO_JUMP` (read-steps only, for thunder, fire and
lava, whose events are sparse and loud by nature — thunder's flagged "clicks"
measured as 1–13 % of the roll's level in the gaps between segments, numerical
dust under hearing, seed-dependent, so a clicks check would be flaky). Wind and
water are unchanged: `analysis::problems` still checks them for everything.

A stop-seam check (`foley-sim stops`, in `all`): each bed is played to its steady
level, freed, and a fresh one given the same scene; the level must not step across
the join, nor the fresh bed fall far quieter. Thunder is left out: its roll is made
on a background thread and doesn't reach a 2 s window, so its row would be vacuous.

Verified: a deliberately broken probe scene (a roof toggling every 2 s at 80 mm/h)
trips `NO_STEP`'s read-step check at 1.58x against 1.50x, so the wiring fails when
it should. The probe was removed after.

A concurrent-mix check (`foley-sim mix`, in `all`): the storm's three beds played
together on one clock, each on its own scene schedule; the sum must keep headroom.
Measured peak −5.5 dBFS, −26.0 LUFS.

## Finding: the wind stop seam — fixed

`stops` reported `wind FAIL | step 8.6 dB (usual 0.7)`: a wind bed stopped and
started again **dipped ~9 dB for ~100 ms** — −25 dB before the join, −35 dB
after, recovering over ~7 frames — whenever a wind bed is restarted: entering or
leaving a cave, a dimension change, toggling the setting, a world load. The same
family as the rain-roof and water-plunge seams already fixed (`9681d62`,
`9406f04`), and wind-specific: water, fire, lava and rain came through the same
check clean.

The mechanism (measured after the fix narrowed it down): `Wind::set_scene` did
set `speed` from the scene on a fresh bed ("the wind is already blowing when
it's first heard"), and the first `gust()` block does read a history value it
has just written — but the leaves' slow envelopes (`leaf_heard`, `rustle_heard`,
`leaf_near`) and the band-passes' state only charge up by actually rendering
samples, so the bed's first ~100 ms was several dB under its steady level.

**Fixed** in `crates/foley-dsp/src/wind.rs`: on a fresh bed's first scene, the
render is warmed up over 8192 discarded samples, so the filters and envelopes
reach steady state before the first sample is heard. `stops` now reads
`wind ok | step 1.4 dB (usual 0.7)`, in line with the other beds. (The Java
side still stops a bed without a fade — `SurroundBeds.stop` — which is a
separate, possible audible cut; route that to a unit/gametest as below.)
