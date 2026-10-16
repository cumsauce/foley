package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.Arrays;
import org.junit.jupiter.api.Test;

/** The settling tick (M22): its delay, its primitive parameters, which materials settle, and its one-shot slot. */
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

	@Test
	void onlyOneSettleSoundsAtATime() {
		// Every render is the same deterministic grain (fixed params, seed 0):
		// two contacts inside the buffer's life queue identical PCM over identical
		// PCM, which combs the tick's 3 kHz band. One slot, one sounding buffer.
		Settling settling = new Settling(null);
		long started = 5_000_000_000L;
		assertTrue(settling.due(started), "the first settle must sound");
		assertFalse(settling.due(started + 1), "a second settle while the first sounds must be dropped");
		assertFalse(settling.due(started + Settling.LIFETIME_MS * 1_000_000L - 1),
				"a settle must not stack on a buffer still sounding");
		assertTrue(settling.due(started + Settling.LIFETIME_MS * 1_000_000L),
				"once the buffer has ended the next settle must sound");
	}

	@Test
	void theSlotLastsAsLongAsTheBuffer() {
		// The slot covers exactly the padded buffer: the head delay plus the tick
		// the primitive renders (PARAMS[2] seconds).
		assertEquals(Settling.DELAY_MS + Math.round(Settling.PARAMS[2] * 1000), Settling.LIFETIME_MS);
	}

	@Test
	void theTickFadesOutInsteadOfCutting() {
		// The grain stops dead on its last sample: a broadband click at the end of
		// a 3 kHz tick. The tail must ramp to silence, body untouched.
		short[] pcm = new short[960];
		Arrays.fill(pcm, (short) 1000);
		Settling.fadeTail(pcm, 48);
		assertEquals(0, pcm[pcm.length - 1], "the last sample must be silent");
		for (int i = 0; i < pcm.length - 48; i++) {
			assertEquals(1000, pcm[i], "the body of the tick must be untouched at " + i);
		}
		assertEquals(1000, pcm[pcm.length - 48], "the fade must start at full level");
		for (int i = pcm.length - 48; i < pcm.length - 1; i++) {
			assertTrue(pcm[i] >= pcm[i + 1], "the tail must ramp down at " + i);
		}
	}
}
