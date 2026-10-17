# Coastline fixes: test-review notes (2026-10-17)

Review of the test changes at `5a7937e`. Three tests were tightened in place (uncommitted, not yet run):
the full-scale onset test now checks from sample 0, `NativeBridgeTest` refuses a negative length, and
`BufferSoundTest` collects reads until end-of-stream instead of assuming one read.

Open items for the implementing agent:

- [x] **Fresh-bed guard starts at 0.0** (`crates/foley-ffi/src/lib.rs`, `FoleyBed` init). Every new bed now
      fades in over ~0.2 s (guard climbs at 0.0001/sample). Starting at `BED_CEILING` blocks the onset leak
      without the fade. If 0.0 stays, playtest that beds arriving (weather start, reaching the shore) don't
      sound late.
      Landed in `8620e02`: fresh beds init at `BED_CEILING`.
- [x] **`surf_0..2` in `sounds.json`.** `GeneratedSoundsTest` pins the surf one-shot arm as removed; drop the
      `surf_0`/`surf_1`/`surf_2` entries from `mod/src/main/resources/assets/foley/sounds.json` in the same
      change, or those events silently fall back to `otherwise`.
      Landed in `ff85934`, which also corrected `docs/PLAN.md`.
- [x] **`leaves_stay_level_matched_at_full_gale` uses one seed** (`Wind::new(5)`) for a peak bound of 0.25 on
      the raw, pre-guard synth. Check it across a few seeds so gusts can't make it flaky.
      Landed in `de23bbb`: bound checked across seeds `[5,11,23,42,777]`, worst 0.2421 against 0.25.
- [x] **`nan_height_is_silent`**: ±∞ → silence, but 1e20 → clamped to 3.0 m. Intentional; keep both, don't
      "unify" them.
      Landed in `702732a`: NaN/±∞ silence and `1e20` ≡ 3.0 m are both pinned.
- [x] **`NativeBridgeTest` tests only `sliceLength`.** Make sure `render()` actually calls it on both the
      first and the retry read (code review; no test reaches it).
      Landed in `41beb6d`: `sliceLength` refuses negative, and `render()` already routed both reads through it.
- [x] **`UnderwaterControllerTest`**: the edited `stayingDryPastTheLingerClearsTheFlag` and the new
      `dry_latch_drops_on_tick_20` overlap. Harmless; merge or keep.
      Landed in `83a4fe3`: `stayingDryPastTheLingerClearsTheFlag` folded into `dry_latch_drops_on_tick_20`.

## Follow-up findings

The measurement work that followed qualified the picture above. `cargo xtask levels` (`6499bea`) renders
10 bed kinds over 27 scenes at 30 s each: **18 scenes exceed 0.4**. Worst are `fire` at 1.3230, `thunder`
at 1.3025, `rain` at 0.8231 and `lava` at 0.5802, with 0.88 % of samples at or above the ceiling.

The guard is an envelope follower, not a limiter. Sparse beds — fire crackles, thunder after silence, lava
pops — pass transients well above `BED_CEILING`, because the envelope is low in the gaps between them.

The summed coastline set (rain + wind + water + surf) peaks at 0.8036 (−1.90 dBFS), so there is 1.9 dB of
headroom before clipping. Wind is now the *quietest* of the loud beds at 0.2436, which retires it as the
leading blast suspect.

Not addressed by any of this: the coastline blast root cause is still unconfirmed. Instrumentation now
exists, though: build with `-Pfoley.audioDebug=true` (`364e8bd`, docs in `docs/audio-debug.md`) and
`grep '^\[foley\] bed='` on the playtest log to see which beds are actually sounding. The open question
remains "does it stop with Foley's wind volume down?".
