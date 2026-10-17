# Coastline fixes: test-review notes (2026-10-17)

Review of the test changes at `5a7937e`. Three tests were tightened in place (uncommitted, not yet run):
the full-scale onset test now checks from sample 0, `NativeBridgeTest` refuses a negative length, and
`BufferSoundTest` collects reads until end-of-stream instead of assuming one read.

Open items for the implementing agent:

- [ ] **Fresh-bed guard starts at 0.0** (`crates/foley-ffi/src/lib.rs`, `FoleyBed` init). Every new bed now
      fades in over ~0.2 s (guard climbs at 0.0001/sample). Starting at `BED_CEILING` blocks the onset leak
      without the fade. If 0.0 stays, playtest that beds arriving (weather start, reaching the shore) don't
      sound late.
- [ ] **`surf_0..2` in `sounds.json`.** `GeneratedSoundsTest` pins the surf one-shot arm as removed; drop the
      `surf_0`/`surf_1`/`surf_2` entries from `mod/src/main/resources/assets/foley/sounds.json` in the same
      change, or those events silently fall back to `otherwise`.
- [ ] **`leaves_stay_level_matched_at_full_gale` uses one seed** (`Wind::new(5)`) for a peak bound of 0.25 on
      the raw, pre-guard synth. Check it across a few seeds so gusts can't make it flaky.
- [ ] **`nan_height_is_silent`**: ±∞ → silence, but 1e20 → clamped to 3.0 m. Intentional; keep both, don't
      "unify" them.
- [ ] **`NativeBridgeTest` tests only `sliceLength`.** Make sure `render()` actually calls it on both the
      first and the retry read (code review; no test reaches it).
- [ ] **`UnderwaterControllerTest`**: the edited `stayingDryPastTheLingerClearsTheFlag` and the new
      `dry_latch_drops_on_tick_20` overlap. Harmless; merge or keep.

Not addressed by any of this: the coastline blast root cause is still unconfirmed (leading suspect: wind
leaves at storm on wooded/high coasts). Needs the instrumented playtest log or the player's answer to
"does it stop with Foley's wind volume down?".
