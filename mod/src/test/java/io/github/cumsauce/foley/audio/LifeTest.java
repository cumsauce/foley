package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Test;

/**
 * Life's surf one-shots are gone (M17): the wave clock, the shore cache and the shore scan moved
 * to {@link SurfController}, which runs one continuous bed instead. The schedule was private
 * state, so the removal is pinned by reflection — Life must declare none of it. The behaviour a
 * shore shows (no {@code surf_*} one-shot fires while the bed is live) is pinned by the client
 * gametest; this is the compile-and-load level guard.
 */
class LifeTest {
	@Test
	void lifeCarriesNoSurfWaveSchedule() {
		assertThrows(NoSuchFieldException.class, () -> Life.class.getDeclaredField("nextWave"));
		assertThrows(NoSuchFieldException.class, () -> Life.class.getDeclaredField("shore"));
		assertThrows(NoSuchFieldException.class, () -> Life.class.getDeclaredField("shoreFoundAt"));
	}

	@Test
	void theSurfTiersStayPublicApi() {
		// GeneratedSounds' addon path renders SURF_M by name; M17 must not take it away.
		assertEquals(3, Life.SURF_M.length);
		assertEquals(0.3f, Life.SURF_M[0], 1e-6);
		assertEquals(1.0f, Life.SURF_M[1], 1e-6);
		assertEquals(2.2f, Life.SURF_M[2], 1e-6);
	}
}
