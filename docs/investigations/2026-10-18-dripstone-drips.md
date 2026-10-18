# Dripstone drips: root-cause investigation

Measured from rendered buffers (`cargo test -p foley-synth --test drip_probe`, 48 kHz mono f32,
written to `/tmp/drip_*.f32`), not from listening.

## The drip into water (`drip_into_water`) — the cave dripstone drip

| variant | length | peak | rms | share of energy in first 10 ms |
| --- | --- | --- | --- | --- |
| 0 (0.6 mm) | 32.0 ms | 0.54 | 0.067 | 0.86 |
| 7 (1.28 mm) | 24.9 ms | 0.69 | 0.124 | 0.95 |
| 15 (3.0 mm) | 50.6 ms | **2.51** | 0.465 | 0.71 |
| seeds 1–3 | 101–139 ms | 0.88 | 0.08 | 0.91 |

Envelope of variant 0, 0.5 ms buckets:

```
 0.0 ms 0.538   <- 2 ms click, full amplitude
 0.5 ms 0.268
 1.0 ms 0.155
 1.5 ms 0.047   <- click gone
 2.0 ms 0.231   <- 4 ms of digital silence
 2.5-4.5 ms ~0.2 <- the bubble rings with a flat, undecaying envelope
```

### Root causes

1. **The loudest thing in the sound is a click, not a drop.** A 2 ms full-amplitude
   down-chirp noise burst at gain up to 1.2 dominates every drip. This is the "jarring" part,
   and it reads as a tick/clip, not water.
2. **~4 ms of exact digital silence** between the click (ends at 2 ms) and the bubble
   (starts at 6 ms). A hard gap inside an 8 ms event is a textbook "lo-fi / broken" artefact.
3. **No peak normalisation and clipping.** Variant 15 peaks at **2.51** — 159 samples are
   clipped by the mixer. Every other one-shot in the file (`bang`) ends with
   `normalize_in_place(&mut out, 0.9)`; the drip never got one, so level varies per variant
   and the loudest one distorts.
4. **Flat-magnitude bubbles that do not decay in the audible time.** Damping is
   π·f·0.045, so a 2.7 kHz bubble has a ~2.6 ms time constant: inside its first 3 ms the
   envelope is flat. The bubble is scheduled *before* the click has decayed, so the two
   additive sources overlap with no decay between them — a solid tone block.
5. **`rng.unit()` is called a variable number of times before the bubble exists**, so the
   bubble radius itself is not pinned the way the doc comment claims ("drips into water
   trap bubbles of about 0.6 to 3 mm"); the dominant pitch jumps between variants and seeds
   (measured ≈ 2.7 kHz, 2.2 kHz, 1.1 kHz) with no consistent relationship to the nominal radius.

## The drip on stone (`drip_on_stone`) — the dripstone drip into a cauldron

| variant | length | peak | rms | share of energy in first 10 ms |
| --- | --- | --- | --- | --- |
| 0 (2.0 mm) | 55.1 ms | 0.52 | 0.024 | 0.994 |
| 7 (2.35 mm) | 60.3 ms | 0.23 | 0.017 | 0.983 |
| 15 (2.8 mm) | 53.2 ms | 0.52 | 0.032 | 0.993 |

### Root causes

6. **99% of the energy is in the first 10 ms.** `Struck`'s stone decay is 900/s (τ ≈ 1.1 ms)
   and the splash has a 4 ms constant, so the remaining 45 ms of buffer is inaudible tail.
   The event reads as a single click, not a drop landing.
7. **Amplitude is not monotonic in radius.** `0.25*(r/0.002)^1.5` gives 0.28 at 2.35 mm but
   0.23 at 2.0 mm and 0.52 at 2.8 mm — the level jitters with the variant rather than
   scaling with drop size.
8. **Dominant energy ≈ 6.2 kHz**, the top of `Struck`'s 3–7 kHz mode pair plus a 9 kHz
   click: bright, thin and hissy, with no low body to sell the mass of a drop.
9. **Mix level is ~10× quieter than the water drip** (rms 0.024 vs 0.24), so in caves the
   two drip families are wildly mismatched.

## What is not the cause

- The `Bubble` model itself is physically sound (correct Minnaert frequency, verified by its
  own test) — it is the *scheduling and normalisation around it* that is wrong.
- The buffer library / sounds.json path is fine: 16 variants per family are requested and
  served (`foley:generated/drip_0..15`, `drip_stone_0..15`).

## The fix (same day)

Each root cause above, and what changed:

1. Click → contact. The entry is a 1.5 ms splash eased in over 0.15 ms, at a twelfth of its
   old amplitude, instead of a 2 ms full-amplitude burst.
2. Dead gap → joined. The bubble starts at 1.2 ms, before the splash fades, so the two are
   one sound. A soft 35 ms wash behind it carries the water settling.
3. Clipping → normalised. Both families end in `fade_into_level`, which peak-normalises
   the buffer and fades the tail off. No variant can clip; measured 0 clipped samples
   across all 32.
4. Flat tone → decay. The bubble now starts under the splash instead of on its own, so it
   decays inside the sound rather than standing as a block.
5. Seed-dependent radius → per-drop. The bubble is always the drop's own radius, so the 16
   variants step through 0.6 → 3.0 mm in pitch order.
6. 99% of energy in 10 ms → real tail. The stone contact is 4 ms with a 20 ms splash
   (was 1.1 ms + 4 ms); 40–90% of its energy is now past 10 ms.
7. Level jitter → one law. Both families use `drip_level(radius)` (0.55 at 0.6 mm, 0.90
   from 1.6 mm), with the stone family varying ±6% per seed so its 16 variants are not
   identically loud.
8. Bright/hissy → softened. The splash slope moved from 2.5 kHz to 1.8 kHz and the
   spatter ticks from 9 kHz to 7 kHz.
9. Mismatched levels → shared law. The water family was also raised (its smallest variant
   sat at 0.16 and was lost under the cave bed); the two now sit together.

**Removed:** the 15% "splash only, no bubble" branch. It produced a 12 ms whisper with no
ring, which read as a glitch (7 of 64 seeds for the largest drip). Every drip now traps a
bubble; variants differ in size and in the droplets that follow.

**Measured after** (all 16 variants per family): water 0.55–0.90 peak, 21–79 ms long,
1–16% of energy past 10 ms; stone 0.79–0.88 peak, 124–128 ms long, 40–91% past 10 ms;
zero clipped samples in any variant.

**Tests:** `drips_are_normalised_continuous_and_sized_by_the_drop` (level is the drop's,
never clips, no silent lead-in, bigger drop is louder) and
`a_drip_has_body_that_outlasts_the_contact` (the 3 mm family lasts past 10 ms for all 16
seeds, and low-band energy beats the 6–8 kHz hiss). Both failed before the fix.

