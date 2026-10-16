package io.github.cumsauce.foley.config;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.google.gson.JsonParser;
import org.junit.jupiter.api.Test;

class FoleyConfigTest {
	@Test
	void itRoundTrips() {
		FoleyConfig changed = new FoleyConfig(40, false, true, false, true, false, true, false, true, false, true,
				false, true, true, false, true, false, true, false, true, (1L << 35) | 0b1010_0101);
		assertEquals(changed, FoleyConfig.fromJson(changed.toJson()));
	}

	@Test
	void missingOrBadValuesFallBackToTheirDefaults() {
		var json = JsonParser.parseString("{\"volume\": 250, \"rain\": \"no\", \"frogs\": false}").getAsJsonObject();
		FoleyConfig read = FoleyConfig.fromJson(json);
		assertEquals(100, read.volume());
		assertEquals(FoleyConfig.DEFAULT.rain(), read.rain());
		assertEquals(false, read.frogs());
		assertEquals(FoleyConfig.DEFAULT.birds(), read.birds());
		assertEquals(0, read.voices());
		var voiced = JsonParser.parseString("{\"voices\": {\"sheep\": true, \"cat\": true, \"dragon\": true}}")
				.getAsJsonObject();
		assertEquals((1 << 2) | (1 << 7), FoleyConfig.fromJson(voiced).voices());
	}

	@Test
	void caveAtmosphereDefaultsOffAndCanBeFlipped() {
		// M12: the cave atmosphere toggle is off by default.
		assertFalse(FoleyConfig.DEFAULT.caveAtmosphere());
		assertFalse(FoleyConfig.NONE.caveAtmosphere());
		assertTrue(FoleyConfig.ALL.caveAtmosphere());
		// withCaveAtmosphere flips just that bit.
		assertTrue(FoleyConfig.DEFAULT.withCaveAtmosphere(true).caveAtmosphere());
		assertFalse(FoleyConfig.ALL.withCaveAtmosphere(false).caveAtmosphere());
	}
}
