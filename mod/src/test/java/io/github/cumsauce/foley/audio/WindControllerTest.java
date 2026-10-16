package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

/**
 * The wind bed's calm: quadrant gains that ease toward each new sample over seconds instead of snapping with every
 * recompute, biased toward a uniform field, and threshold hysteresis so a listener hovering at still air or shelter
 * doesn't hear the bed cut out and back in (the twin of {@link CaveControllerTest}).
 */
class WindControllerTest {
	/** Biased 40% toward a quarter each: 1 leans to 0.7, 0 to 0.1, 0.5 to 0.4. */
	private static final float[] LOUD_QUARTER = {0.7f, 0.1f, 0.1f, 0.1f};
	private static final float[] QUIET_FIELD = {0.1f, 0.1f, 0.1f, 0.1f};

	@Test
	void firstStepInitializesOnTheBiasedTargetWithoutNaN() {
		WindController.Pan pan = new WindController.Pan();
		float[] eased = pan.step(new float[] {1, 0, 0, 0});
		assertArrayEquals(LOUD_QUARTER, eased, 1e-6f, "first sample should land on the biased target, not drift up from zero");
		for (float gain : eased) assertTrue(Float.isFinite(gain), "non-finite gain");
		pan.reset();
		assertArrayEquals(QUIET_FIELD, pan.step(new float[SurroundBeds.AROUND]), 1e-6f, "reset should re-initialise");
		for (float gain : pan.step(new float[] {0.5f, 0.5f, 0.5f, 0.5f})) assertTrue(Float.isFinite(gain), "non-finite gain");
	}

	@Test
	void easedGainsDriftTowardTheTargetWithoutEverPassingIt() {
		WindController.Pan pan = new WindController.Pan();
		pan.step(new float[] {1, 0, 0, 0}); // lands on {0.7, 0.1, 0.1, 0.1}
		float[] target = {0, 1, 0, 0};
		float[] biasedTarget = {0.1f, 0.7f, 0.1f, 0.1f};
		float[] was = pan.step(target).clone();
		float[] now = pan.step(target).clone();
		for (int q = 0; q < now.length; q++) {
			if (biasedTarget[q] == was[q]) { // already there: nothing to chase, nothing to do
				assertEquals(was[q], now[q], 0f, "quarter " + q + " crept with its target unchanged");
				continue;
			}
			assertTrue((now[q] - was[q]) * (biasedTarget[q] - was[q]) > 0,
					"quarter " + q + " did not move toward its target");
			assertTrue(Math.abs(now[q] - biasedTarget[q]) < Math.abs(was[q] - biasedTarget[q]),
					"quarter " + q + " did not get closer to its target");
		}
		for (int i = 0; i < 60; i++) pan.step(target); // 15 s of samples at 4 Hz
		assertArrayEquals(biasedTarget, pan.step(target), 0.02f, "did not settle within 2% after 15 s");
	}

	@Test
	void anUnchangedTargetStaysPut() {
		WindController.Pan pan = new WindController.Pan();
		float[] sample = {0.2f, 0.9f, 0.1f, 0.4f};
		float[] settled = pan.step(sample).clone();
		assertArrayEquals(settled, pan.step(sample), 0f, "gains crept with an unchanged sample");
		assertArrayEquals(settled, pan.step(sample), 0f, "gains crept with an unchanged sample");
	}

	@Test
	void theBiasCompressesTheSpreadTowardUniform() {
		float spread = WindController.center(1, 0.4f) - WindController.center(0, 0.4f);
		assertTrue(spread < 1, "bias did not compress the spread: " + spread);
		// The lean the trees put in survives: loud stays louder than quiet.
		assertTrue(WindController.center(1, 0.4f) > WindController.center(0.5f, 0.4f));
		assertTrue(WindController.center(0.5f, 0.4f) > WindController.center(0, 0.4f));
		// An even field is a fixed point.
		assertEquals(0.25f, WindController.center(0.25f, 0.4f), 0f);
	}

	@Test
	void speedHoveringAtTheStopThresholdDoesNotFlapARunningWindBed() {
		// Latch on in a breeze, then sink toward still: the band between STILL_STOP and STILL is hysteresis.
		boolean heard = WindController.heard(false, 1.2f, 0, 0);
		assertTrue(heard, "did not start clearly blowing");
		float[] sink = {0.45f, 0.3f, 0.4f, 0.28f, 0.35f, 0.31f};
		for (int i = 0; i < sink.length; i++) {
			heard = WindController.heard(heard, sink[i], 0, 0);
			assertTrue(heard, "bed flapped off at speed " + sink[i] + " (sample " + i + ")");
		}
		assertFalse(WindController.heard(heard, 0.1f, 0, 0), "did not stop in clearly still air");
	}

	@Test
	void aStoppedBedDoesNotStartOnTheHysteresisBand() {
		// The band only stabilises a running bed: it must not start one out of still air.
		assertFalse(WindController.heard(false, 0.45f, 0, 0), "started below the still threshold");
		assertTrue(WindController.heard(false, 0.5f, 0, 0), "did not start at the still threshold");
	}

	@Test
	void exposureHoveringAtTheShutInThresholdDoesNotFlapARunningWindBed() {
		boolean heard = WindController.heard(false, 0, 0.05f, 0);
		assertTrue(heard, "did not start with a breath of air through the shelter");
		float[] wander = {0.015f, 0.012f, 0.018f, 0.011f, 0.016f}; // between SHUT_IN_STOP and SHUT_IN
		for (int i = 0; i < wander.length; i++) {
			heard = WindController.heard(heard, 0, wander[i], 0);
			assertTrue(heard, "bed flapped off at exposure " + wander[i] + " (sample " + i + ")");
		}
		assertFalse(WindController.heard(heard, 0, 0.004f, 0), "did not stop clearly shut in");
		assertFalse(WindController.heard(false, 0, 0.015f, 0), "started on the hysteresis band");
	}

	@Test
	void leavesHoveringAtTheThresholdDoesNotFlapARunningWindBed() {
		// In still air among leaves the bed runs on the leaves alone; the band keeps it up as they thin.
		boolean heard = WindController.heard(false, 0, 0, 0.1f);
		assertTrue(heard, "did not start on the leaves alone");
		float[] wander = {0.04f, 0.03f, 0.045f, 0.028f, 0.035f}; // between LEAVES_STOP and LEAVES
		for (int i = 0; i < wander.length; i++) {
			heard = WindController.heard(heard, 0, 0, wander[i]);
			assertTrue(heard, "bed flapped off at leaves " + wander[i] + " (sample " + i + ")");
		}
		assertFalse(WindController.heard(heard, 0, 0, 0.01f), "did not stop with the leaves gone");
		assertFalse(WindController.heard(false, 0, 0, 0.049f), "started below the leaves threshold");
		assertTrue(WindController.heard(false, 0, 0, 0.05f), "did not start at the leaves threshold");
	}
}
