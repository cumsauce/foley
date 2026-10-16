package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

/** The settling tick (M22): its delay, its primitive parameters, and which materials settle. */
class SettlingTest {
	@Test
	void theDelayIsInTheSpecsWindow() {
		// DELAY_MS must be in the spec's 10 to 30 ms window so the tick sits
		// behind the M4 impact, not under it.
		int delay = Settling.DELAY_MS;
		assertTrue(delay >= 10 && delay <= 30, "delay out of window: " + delay);
	}

	@Test
	void theParamsMatchTheFOLEY_PRIMITIVE_NOISERanges() {
		// Centre 3000 Hz (20 to 20000), q 1.5 (0.5 to 20), seconds 0.02
		// (0.005 to 5), gain 0.05 (0 to 1) — the spec's settling grain.
		float[] p = Settling.PARAMS;
		assertEquals(4, p.length);
		assertTrue(p[0] >= 20 && p[0] <= 20_000, "centre: " + p[0]);
		assertTrue(p[1] >= 0.5 && p[1] <= 20, "q: " + p[1]);
		assertTrue(p[2] >= 0.005 && p[2] <= 5, "seconds: " + p[2]);
		assertTrue(p[3] >= 0 && p[3] <= 1, "gain: " + p[3]);
	}

	@Test
	void onlyGravelSandAndSnowSettle() {
		// The settle tick is for granular shifts and snow brushes — not a second
		// heel-strike on stone or wood.
		for (int m : new int[] {Materials.STONE, Materials.WOOD, Materials.METAL, Materials.ICE,
				Materials.PLANT, Materials.SOFT}) {
			assertEquals(false, Settling.shouldSettle(m), "settles on " + m);
		}
		for (int m : new int[] {Materials.GRANULAR, Materials.EARTH, Materials.SNOW}) {
			assertEquals(true, Settling.shouldSettle(m), "misses " + m);
		}
	}
}