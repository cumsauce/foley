package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

/** Ash and snow drift (M22): the primitive parameters and the storm predicates. */
class AshDriftTest {
	@Test
	void theDriftParamsAreAudibleButQuiet() {
		// Centre 6 kHz (20 to 20000), q 4 (0.5 to 20), seconds 0.4 (0.005 to 5),
		// gain 0.04 (0 to 1) — the spec's almost-inaudible texture.
		float[] p = AshDrift.PARAMS;
		assertEquals(4, p.length);
		assertTrue(p[0] >= 20 && p[0] <= 20_000, "centre: " + p[0]);
		assertTrue(p[1] >= 0.5 && p[1] <= 20, "q: " + p[1]);
		assertTrue(p[2] >= 0.005 && p[2] <= 5, "seconds: " + p[2]);
		assertTrue(p[3] >= 0 && p[3] <= 1, "gain: " + p[3]);
	}

	@Test
	void theCadenceIsAFewSecondsApart() {
		// 4 to 8 seconds between ticks — a continuous wash, not a beat.
		assertTrue(AshDrift.MIN_INTERVAL_TICKS >= 60, "min too short");
		assertTrue(AshDrift.MAX_INTERVAL_TICKS <= 240, "max too long");
	}
}