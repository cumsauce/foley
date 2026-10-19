package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

/**
 * The underwater bed's two gates. The submerged flag (C1) and its hysteresis (B2): the native
 * SUBMERGED flag low-passes every bed at 1 kHz, so it must follow the underwater config toggle as
 * well as actual submersion, and a listener bobbing at the surface must not flap it. And the bed's
 * level: how deep the ear sits under the water <em>surface</em>. The level used to be measured
 * under the world's sea level instead, so a swimmer in a mountain lake (sea 62, ear at 100) and a
 * swimmer in a superflat pond (sea −63, ear at −58) both read "above sea level" and the bed never
 * played at all — while a one-block puddle in a deep cave read a full 1.0.
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
	void dry_latch_drops_on_tick_20() {
		// The dry counter as tick() advances it, with the latch: a wet ear latches at once; then a
		// fully dry run must hold the flag through the linger and let go on the 20th dry tick —
		// DRY_LINGER's documented meaning — not the 21st. The direct boundaries below pin an
		// already-latched flag at 1 and DRY_LINGER - 1 dry ticks, independent of the counter.
		assertTrue(
				UnderwaterController.latchedSubmerged(true, false, 1), "latched flag cleared after only one dry tick");
		assertTrue(
				UnderwaterController.latchedSubmerged(true, false, UnderwaterController.DRY_LINGER - 1),
				"latched flag cleared one tick before the linger");
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

	/** The run-18 gametest tank, exactly: superflat water capped at y=-52, ear at -58.38, sea −63. */
	@Test
	void theSuperflatTankIsAudibleWhereSeaLevelWouldNotHearIt() {
		float level = UnderwaterController.depthLevel(-52f, -58.38f);
		assertTrue(level > 0f, "a sealed tank with the ear six blocks under its surface read level " + level);
	}

	@Test
	void aLakeHighAboveSeaLevelIsStillAudible() {
		// The ear half a block under a surface at y=100: the old formula handed back
		// max(0, 62 - 100.5) = 0 and silenced the bed for every body of water above y=62.
		assertTrue(UnderwaterController.depthLevel(101f, 100.5f) > 0f,
				"a swimmer just under a mountain lake's surface was silent");
	}

	@Test
	void depthScalesLinearlyAndSaturatesAtMaxDepth() {
		// MAX_DEPTH is 30 blocks: 15 under is half, 30 or more under is full.
		assertEquals(0.5f, UnderwaterController.depthLevel(62f, 47f), 1e-6f);
		assertEquals(1f, UnderwaterController.depthLevel(62f, 32f), 1e-6f);
		assertEquals(1f, UnderwaterController.depthLevel(100f, 60f), 1e-6f);
	}

	@Test
	void anEarAboveTheSurfaceIsSilent() {
		// The ear a block and a half out of the water: level 0, never negative.
		assertEquals(0f, UnderwaterController.depthLevel(99f, 100.5f), 0f);
	}
}
