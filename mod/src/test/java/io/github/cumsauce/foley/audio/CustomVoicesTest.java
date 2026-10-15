package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.mojang.serialization.JsonOps;
import com.google.gson.JsonParser;
import io.github.cumsauce.foley.api.VoiceDescriptor;
import org.junit.jupiter.api.Test;

/** F3's creature voices: the descriptor's checks and its codec. */
class CustomVoicesTest {
	@Test
	void descriptorsRoundTripThroughJson() {
		VoiceDescriptor dolphin = new VoiceDescriptor(0.3, 300, 0.4, 0.6);
		VoiceDescriptor read = VoiceDescriptor.CODEC.decoder()
				.parse(JsonOps.INSTANCE,
						VoiceDescriptor.CODEC.encoder().encodeStart(JsonOps.INSTANCE, dolphin).getOrThrow())
				.getOrThrow();
		assertEquals(dolphin, read);
		VoiceDescriptor defaults = VoiceDescriptor.CODEC.decoder()
				.parse(JsonOps.INSTANCE, JsonParser.parseString(
						"{ \"tract_length_m\": 0.17, \"pitch_hz\": 120 }")).getOrThrow();
		assertEquals(new VoiceDescriptor(0.17, 120, 0.5, 0.5), defaults, "rough and breathy default to halfway");
	}

	@Test
	void descriptorsAreCheckedWhereBuilt() {
		for (Runnable bad : new Runnable[] {
				() -> new VoiceDescriptor(2.0, 120, 0.5, 0.5),
				() -> new VoiceDescriptor(0.17, 20, 0.5, 0.5),
				() -> new VoiceDescriptor(0.17, 120, 1.5, 0.5),
				() -> new VoiceDescriptor(0.17, 120, 0.5, -0.1)}) {
			assertThrows(IllegalArgumentException.class, bad::run);
		}
	}

	@Test
	void theFloatsMatchTheNativesOrder() {
		float[] floats = new VoiceDescriptor(0.3, 300, 0.4, 0.6).toFloats();
		assertEquals(4, floats.length);
		assertEquals(0.3f, floats[0], 1e-6f);
		assertEquals(300f, floats[1], 1e-3f);
	}

	@Test
	void archetypeNamesAreTheNatives() {
		assertEquals(5, CustomVoices.ARCHETYPES.size());
		assertEquals(0, CustomVoices.archetype("ambient"));
		assertEquals(4, CustomVoices.archetype("breath"));
		assertTrue(CustomVoices.archetype("roar") < 0);
	}
}
