package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Map;
import org.junit.jupiter.api.Test;

/**
 * Every built-in bed has to resolve through sounds.json: {@link BedSound} is not self-resolving, so
 * the game looks {@code foley:<name>} up there before it ever opens the native stream. An entry that
 * is missing entirely — or present but not marked {@code stream: true} — leaves the game with a
 * static sound (or none), so the synthesized bed never renders and ships silent. Both the cave and
 * the underwater bed went out that way; this keeps the whole rack honest. The names come from the
 * controller that builds each bed.
 */
class BedSoundEventsTest {
	private static final Map<String, String> BEDS = new LinkedHashMap<>();

	static {
		BEDS.put("rain", "RainController");
		BEDS.put("wind", "WindController");
		BEDS.put("biome", "BiomeController");
		BEDS.put("water", "PlacedBeds");
		BEDS.put("fire", "PlacedBeds");
		BEDS.put("lava", "PlacedBeds");
		BEDS.put("thunder", "ThunderController");
		BEDS.put("surf", "SurfController");
		BEDS.put("cave", "CaveController");
		BEDS.put("underwater", "UnderwaterController");
	}

	@Test
	void everyBuiltInBedStreamsItsSound() throws Exception {
		JsonObject json;
		try (InputStream in = BedSoundEventsTest.class.getResourceAsStream("/assets/foley/sounds.json")) {
			assertNotNull(in, "sounds.json is not on the test classpath");
			json = JsonParser.parseReader(new InputStreamReader(in, StandardCharsets.UTF_8)).getAsJsonObject();
		}
		for (var bed : BEDS.entrySet()) {
			String name = bed.getKey();
			assertTrue(json.has(name), "foley:" + name + " (" + bed.getValue()
					+ ") has no sounds.json entry, so the game cannot resolve the bed and it never plays");
			boolean streaming = false;
			for (var sound : json.getAsJsonObject(name).getAsJsonArray("sounds")) {
				if (sound.isJsonObject() && sound.getAsJsonObject().has("stream")
						&& sound.getAsJsonObject().get("stream").getAsBoolean()) streaming = true;
			}
			assertTrue(streaming, "foley:" + name + " (" + bed.getValue()
					+ ") is not marked stream:true, so the game loads a static ogg and the synthesized bed never runs");
		}
	}
}
