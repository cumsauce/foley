package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

/**
 * The submerged gate (C1) and its hysteresis (B2): the native SUBMERGED flag low-passes every bed at
 * 1 kHz, so it must follow the underwater config toggle as well as actual submersion, and a listener
 * bobbing at the surface must not flap it.
 */
class UnderwaterControllerTest {
	@Test
	void configOffAndWetLeavesTheFlagFalse() {
		// The bug: submerged was set from scene.level() > 0 alone, so with the underwater
		// toggle off every other Foley bed still got the 1 kHz low-pass near water.
		assertFalse(UnderwaterController.submergedFlag(false, true), "config off && wet must not raise the flag");
	}

	@Test
	void theFlagFollowsConfigAndSubmersion() {
		assertTrue(UnderwaterController.submergedFlag(true, true), "config on && wet raises the flag");
		assertFalse(UnderwaterController.submergedFlag(true, false), "config on && dry stays clear");
		assertFalse(UnderwaterController.submergedFlag(false, false), "config off && dry stays clear");
	}

	@Test
	void bobbingAtTheSurfaceDoesNotFlapTheFlag() {
		// A synthetic wet/dry alternation (waves lapping over the ear, one sample apart) with the
		// linger never exhausted: the flag must latch on at the first wet sample and stay on.
		boolean flag = false;
		int dryFor = 0;
		boolean[] wetSeries = {true, false, true, false, true, false, true, false};
		for (int i = 0; i < wetSeries.length; i++) {
			boolean wet = wetSeries[i];
			dryFor = wet ? 0 : dryFor + 1;
			flag = UnderwaterController.latchedSubmerged(flag, wet, dryFor);
			assertTrue(flag, "flapped off at sample " + i + " (a dry frame inside the linger)");
		}
	}

	@Test
	void stayingDryPastTheLingerClearsTheFlag() {
		// Latched on, then fully dry: the flag holds through the linger, then lets go on the 20th tick.
		boolean flag = true;
		for (int dryFor = 1; dryFor < UnderwaterController.DRY_LINGER; dryFor++) {
			flag = UnderwaterController.latchedSubmerged(flag, false, dryFor);
			assertTrue(flag, "cleared after only " + dryFor + " dry ticks, before the linger");
		}
		flag = UnderwaterController.latchedSubmerged(flag, false, UnderwaterController.DRY_LINGER);
		assertFalse(flag, "still set on the 20th dry tick — the linger is up");
	}

	@Test
	void dry_latch_drops_on_tick_20() {
		// The dry counter as tick() advances it, with the latch: a wet ear latches at once; then a
		// fully dry run must hold the flag through the linger and let go on the 20th dry tick —
		// DRY_LINGER's documented meaning — not the 21st.
		boolean flag = UnderwaterController.latchedSubmerged(false, true, 0);
		assertTrue(flag, "wet must latch");
		int dryFor = 0;
		for (int tick = 1; tick < UnderwaterController.DRY_LINGER; tick++) {
			dryFor = UnderwaterController.dryTicks(false, dryFor);
			flag = UnderwaterController.latchedSubmerged(flag, false, dryFor);
			assertTrue(flag, "latch dropped at dry tick " + tick + ", before the linger");
		}
		dryFor = UnderwaterController.dryTicks(false, dryFor); // the 20th dry tick
		flag = UnderwaterController.latchedSubmerged(flag, false, dryFor);
		assertFalse(flag, "latch still set on dry tick " + UnderwaterController.DRY_LINGER);
		// A wet tick relatches at once.
		assertTrue(UnderwaterController.latchedSubmerged(flag, true, 0), "re-wetting must re-latch");
	}

	@Test
	void dryFromTheStartNeverLatches() {
		assertFalse(UnderwaterController.latchedSubmerged(false, false, 0), "never-wet latched");
		assertFalse(UnderwaterController.latchedSubmerged(false, false, 999), "never-wet latched at large dryFor");
	}
}
