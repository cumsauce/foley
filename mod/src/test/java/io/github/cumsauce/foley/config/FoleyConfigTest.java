package io.github.cumsauce.foley.config;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.google.gson.JsonParser;
import org.junit.jupiter.api.Test;

class FoleyConfigTest {
	@Test
	void itRoundTrips() {
		FoleyConfig changed = new FoleyConfig(40, false, true, false, true, false, true, false, true, false, true,
				false, true);
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
	}
}
