package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

/**
 * The cave bed's threshold hysteresis (B2): the old gate stopped the whole bed the moment depth or
 * openness crossed a single threshold, so a listener hovering at a cave mouth (openness fed from the
 * wind scene, which chatters near the threshold) cut the bed out and back in every sample.
 */
class CaveControllerTest {
	@Test
	void opennessHoveringAtTheThresholdDoesNotFlapARunningBed() {
		// Latch on clearly shut in, then wander across the old 0.2 threshold — the band between
		// SHUT_IN and SHUT_IN_STOP is hysteresis: the bed must stay up through it.
		boolean heard = CaveController.heard(false, 20, 0.15f);
		assertTrue(heard, "did not start clearly shut in");
		float[] wander = {0.21f, 0.19f, 0.25f, 0.22f, 0.28f, 0.24f, 0.19f};
		for (int i = 0; i < wander.length; i++) {
			heard = CaveController.heard(heard, 20, wander[i]);
			assertTrue(heard, "bed flapped off at openness " + wander[i] + " (sample " + i + ")");
		}
	}

	@Test
	void clearlyOpenStopsTheBedAndItStaysDownUntilClearlyShutInAgain() {
		boolean heard = CaveController.heard(false, 20, 0.15f);
		heard = CaveController.heard(heard, 20, 0.35f);
		assertFalse(heard, "did not stop well above the exit threshold");
		// Just under the entry threshold again is not enough to restart a stopped bed? It is — entry
		// uses the tight threshold — but hovering just above it must keep it down.
		assertFalse(CaveController.heard(false, 20, 0.25f), "restarted while still too open");
		assertTrue(CaveController.heard(false, 20, 0.2f), "did not restart at the entry threshold");
	}

	@Test
	void depthHoveringAtTheThresholdDoesNotFlapARunningBed() {
		boolean heard = CaveController.heard(false, 12, 0.1f);
		assertTrue(heard, "did not start deep enough");
		int[] wander = {7, 6, 7, 8, 6, 7};
		for (int i = 0; i < wander.length; i++) {
			heard = CaveController.heard(heard, wander[i], 0.1f);
			assertTrue(heard, "bed flapped off at depth " + wander[i] + " (sample " + i + ")");
		}
		assertFalse(CaveController.heard(heard, 5, 0.1f), "did not stop clearly shallow");
	}

	@Test
	void aStoppedBedDoesNotStartOnTheHysteresisBand() {
		// The band only stabilises a running bed: it must not start one out in the open or too shallow.
		assertFalse(CaveController.heard(false, 20, 0.21f), "started above the entry openness");
		assertFalse(CaveController.heard(false, 7, 0.1f), "started above the entry depth");
	}
}
