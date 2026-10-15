package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.mojang.serialization.JsonOps;
import io.github.cumsauce.foley.api.SoundCodecs;
import io.github.cumsauce.foley.api.SoundModel;
import java.util.List;
import java.util.Map;
import net.minecraft.resources.Identifier;
import org.junit.jupiter.api.Test;

/** The integration API's data: every model round-trips, and the registry keeps addons to their own namespace. */
class AddonSoundsTest {
	private static SoundModel roundTrip(SoundModel model) {
		return SoundCodecs.CODEC.parse(JsonOps.INSTANCE,
				SoundCodecs.CODEC.encodeStart(JsonOps.INSTANCE, model).getOrThrow()).getOrThrow();
	}

	@Test
	void everyModelRoundTrips() {
		List<SoundModel> models = List.of(new SoundModel.DripIntoWater(1.2f), new SoundModel.DripOnStone(2.4f),
				new SoundModel.LavaDrip(), new SoundModel.Crickets(22f), new SoundModel.Birdsong(5),
				new SoundModel.Frog(8f), new SoundModel.Surf(1.5f), new SoundModel.Woodpecker(), new SoundModel.Owl(),
				new SoundModel.Cicadas(30f), new SoundModel.Howl(), new SoundModel.Brush(SoundModel.PlantKind.LEAVES),
				new SoundModel.IceSinging(), new SoundModel.SettlingStone(), new SoundModel.Flush(),
				new SoundModel.Buzz(SoundModel.InsectKind.FLY),
				new SoundModel.Contact(SoundModel.ContactKind.STEP, SoundModel.MaterialKind.SNOW, 1f),
				new SoundModel.Voice("cow", "ambient"), new SoundModel.Bubble(1.5f),
				new SoundModel.NoiseBurst(2000f, 1.2f, 0.3f, 0.8f), new SoundModel.Tone(440f, 880f, 0.5f, 0.6f),
				new SoundModel.Gust(15f, 3f, 1f));
		for (SoundModel model : models) {
			assertEquals(model, roundTrip(model), model.toString());
		}
	}

	@Test
	void jsonNamesWhatItReads() {
		JsonObject json = JsonParser.parseString("""
				{ "model": "drip_into_water", "radius_mm": 3.0 }
				""").getAsJsonObject();
		assertEquals(new SoundModel.DripIntoWater(3f), SoundCodecs.CODEC.parse(JsonOps.INSTANCE, json).getOrThrow());
		assertEquals("model", json.keySet().iterator().next(), "the discriminator comes first, for plain readers");
		assertEquals("drip_into_water", json.get("model").getAsString());
	}

	@Test
	void badModelsSayWhy() {
		assertTrue(SoundCodecs.CODEC.parse(JsonOps.INSTANCE,
				JsonParser.parseString("{ \"radius_mm\": 1 }")).error().isPresent(), "no model");
		assertTrue(SoundCodecs.CODEC.parse(JsonOps.INSTANCE,
				JsonParser.parseString("{ \"model\": \"thunder\" }")).error().isPresent(), "unknown model");
		assertTrue(SoundCodecs.CODEC.parse(JsonOps.INSTANCE,
				JsonParser.parseString("{ \"model\": \"drip_into_water\", \"radius_mm\": 50 }")).error().isPresent(),
				"radius out of range");
		assertTrue(SoundCodecs.CODEC.parse(JsonOps.INSTANCE,
				JsonParser.parseString("{ \"model\": \"brush\", \"plant\": \"iron\" }")).error().isPresent(),
				"unknown plant");
		assertThrows(IllegalArgumentException.class, () -> new SoundModel.DripIntoWater(50f));
		assertThrows(IllegalArgumentException.class, () -> new SoundModel.Voice("Cow", "ambient"));
		assertThrows(IllegalArgumentException.class, () -> new SoundModel.Bubble(50f), "radius out of range");
		assertThrows(IllegalArgumentException.class, () -> new SoundModel.NoiseBurst(2000f, 1.2f, 0.3f, 2f),
				"gain out of range");
		assertThrows(IllegalArgumentException.class, () -> new SoundModel.Tone(440f, 880f, 9f, 0.5f),
				"seconds out of range");
		assertThrows(IllegalArgumentException.class, () -> new SoundModel.Gust(80f, 3f, 1f), "speed out of range");
	}

	@Test
	void entriesReadTheirVariants() {
		assertEquals(new SoundModel.DripOnStone(2.2f),
				AddonSounds.parseEntry(
						JsonParser.parseString("{ \"model\": \"drip_on_stone\", \"radius_mm\": 2.2 }").getAsJsonObject(),
						"test").model());
		assertEquals(8, AddonSounds.parseEntry(
				JsonParser.parseString("{ \"model\": \"howl\" }").getAsJsonObject(), "test").variants());
		assertThrows(RuntimeException.class, () -> AddonSounds.parseEntry(
				JsonParser.parseString("{ \"model\": \"howl\", \"variants\": 65 }").getAsJsonObject(), "test"));
	}

	@Test
	void addonsKeepToTheirOwnNamespace() {
		AddonSounds.register("mymod", Identifier.fromNamespaceAndPath("mymod", "a_drip"),
				new SoundModel.DripIntoWater(1f), 4);
		assertNotNull(AddonSounds.find(Identifier.fromNamespaceAndPath("mymod", "a_drip")));
		assertThrows(IllegalArgumentException.class,
				() -> AddonSounds.register("othermod", Identifier.fromNamespaceAndPath("mymod", "a_drip"),
						new SoundModel.LavaDrip(), 4),
				"a sound's namespace is its owner's");
		assertThrows(IllegalArgumentException.class,
				() -> AddonSounds.register("mymod", Identifier.fromNamespaceAndPath("mymod", "a_drip"),
						new SoundModel.LavaDrip(), 4),
				"duplicate");
		assertThrows(IllegalArgumentException.class,
				() -> AddonSounds.register("mymod", Identifier.fromNamespaceAndPath("mymod", "too_many"),
						new SoundModel.LavaDrip(), 65),
				"variants out of range");
		assertThrows(IllegalArgumentException.class,
				() -> AddonSounds.register("mymod", Identifier.fromNamespaceAndPath("mymod", "a_dragon"),
						new SoundModel.Voice("dragon", "ambient"), 4),
				"unknown mob");
	}

	@Test
	void codesDefinitionWinsOverThePacks() {
		AddonSounds.register("mymod", Identifier.fromNamespaceAndPath("mymod", "mine"), new SoundModel.Howl(), 2);
		AddonSounds.fromPacks(Map.of(Identifier.fromNamespaceAndPath("mymod", "mine"), new AddonSounds.Entry(
				new SoundModel.Owl(), 2, "a pack"), Identifier.fromNamespaceAndPath("mymod", "theirs"),
				new AddonSounds.Entry(new SoundModel.Owl(), 2, "a pack")));
		assertEquals(new SoundModel.Howl(), AddonSounds.find(Identifier.fromNamespaceAndPath("mymod", "mine")).model(),
				"the code's is kept");
		assertEquals(new SoundModel.Owl(), AddonSounds.find(Identifier.fromNamespaceAndPath("mymod", "theirs")).model());
		// Back to no packs, so one test's leftovers don't leak into another's.
		AddonSounds.fromPacks(Map.of());
	}
}
