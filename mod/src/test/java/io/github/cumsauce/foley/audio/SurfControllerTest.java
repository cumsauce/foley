package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

/**
 * The surf bed's pure helpers (M17): the three wave-height tiers Life's one-shot picked from
 * {@link Life#SURF_M}, and the wind exposure the bed takes. The sampler itself needs a live
 * client, so it is covered by the client gametest, not here.
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
	void noWaterMeansNoHeight() {
		assertEquals(0f, SurfScene.SILENT.height(), 1e-6);
	}

	@Test
	void exposureScalesTheWindUpToFullStormSpeed() {
		assertEquals(0f, SurfController.exposureFor(0f), 1e-6);
		assertEquals(0.3125f, SurfController.exposureFor(WindScene.CLEAR), 1e-6);
		assertEquals(0.5f, SurfController.exposureFor(8f), 1e-6);
		assertEquals(1f, SurfController.exposureFor(WindScene.STORM), 1e-6);
	}

	@Test
	void exposureClampsBeyondFullStormAndBelowStillAir() {
		assertEquals(1f, SurfController.exposureFor(60f), 1e-6);
		assertEquals(0f, SurfController.exposureFor(-1f), 1e-6);
	}

	@Test
	void a_probe_miss_holds_the_last_scene_for_a_few_samples_then_silences() {
		assertTrue(SurfController.holdsOnMiss(0), "a first miss holds the scene");
		assertTrue(SurfController.holdsOnMiss(1), "a second miss holds the scene");
		assertTrue(SurfController.holdsOnMiss(2), "a third miss holds the scene");
		assertFalse(SurfController.holdsOnMiss(3), "the third consecutive miss silences the bed");
	}
}
