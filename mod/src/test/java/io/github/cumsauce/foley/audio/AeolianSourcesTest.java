package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;
import org.junit.jupiter.api.Test;

class AeolianSourcesTest {
	@Test
	void silenceThresholdIsTwoMetresPerSecond() {
		assertEquals(2.0f, AeolianSources.silenceThreshold(), 0.0001f);
	}

	@Test
	void strouhalPitchForTypicalDiameters() {
		// d = 40 mm, U = 10 m/s -> 0.2 * 10 / 0.040 = 50 Hz
		assertEquals(50.0f, AeolianSources.pitch(0.040f, 10.0f), 0.0001f);
		// d = 25 mm, U = 10 m/s -> 80 Hz
		assertEquals(80.0f, AeolianSources.pitch(0.025f, 10.0f), 0.0001f);
		// d = 150 mm (bell), U = 10 m/s -> ~13.3 Hz
		assertEquals(13.3333f, AeolianSources.pitch(0.150f, 10.0f), 0.01f);
	}

	@Test
	void smallerDiameterPitchesHigher() {
		assertTrue(AeolianSources.pitch(0.025f, 10.0f) > AeolianSources.pitch(0.150f, 10.0f));
	}

	@Test
	void harderWindPitchesHigher() {
		assertTrue(AeolianSources.pitch(0.040f, 15.0f) > AeolianSources.pitch(0.040f, 5.0f));
	}
}