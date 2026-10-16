package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

/**
 * The surf bed's pure helpers (M17): the three wave-height tiers Life's one-shot picked from
 * {@link Life#SURF_M}, and the surge rate the native bed's grain schedule mirrors. The sampler
 * itself needs a live client, so it is covered by the client gametest, not here.
 */
class SurfControllerTest {
	@Test
	void saltWaterRaisesTheHeightLakeDoesNot() {
		assertEquals(0.3f, SurfController.heightFor(false, false), 1e-6);
		assertEquals(1.0f, SurfController.heightFor(true, false), 1e-6);
		assertEquals(2.2f, SurfController.heightFor(true, true), 1e-6);
	}

	@Test
	void aStormRaisesTheHeightOnlyForSaltWater() {
		assertEquals(SurfController.heightFor(false, false), SurfController.heightFor(false, true), 1e-6);
	}

	@Test
	void surgeRateRisesWithStormAndWind() {
		assertTrue(SurfController.surgeRate(1f, 1f) > SurfController.surgeRate(0f, 0f));
		assertTrue(SurfController.surgeRate(1f, 0f) > 0f);
	}

	@Test
	void noWaterMeansNoHeight() {
		assertEquals(0f, SurfScene.SILENT.height(), 1e-6);
	}
}
