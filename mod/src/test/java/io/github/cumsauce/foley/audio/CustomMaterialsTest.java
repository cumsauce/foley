package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.mojang.serialization.JsonOps;
import com.google.gson.JsonParser;
import io.github.cumsauce.foley.api.MaterialDescriptor;
import io.github.cumsauce.foley.api.SoundModel;
import net.minecraft.resources.Identifier;
import org.junit.jupiter.api.Test;

/**
 * F1's custom materials: the descriptor's checks and its codec, and the registry's rules as far as they reach without
 * the game's registries (blocks and packs are the gametest's).
 */
class CustomMaterialsTest {
	@Test
	void descriptorsRoundTripThroughJson() {
		MaterialDescriptor crystal = new MaterialDescriptor(SoundModel.MaterialKind.GLASS, 90e9, 3100, 0.2, 0.01,
				0.05, 24, 0.5, 0.003);
		MaterialDescriptor read = MaterialDescriptor.CODEC.decoder()
				.parse(JsonOps.INSTANCE, MaterialDescriptor.CODEC.encoder().encodeStart(JsonOps.INSTANCE, crystal)
						.getOrThrow())
				.getOrThrow();
		assertEquals(crystal, read);
		MaterialDescriptor fromJson = MaterialDescriptor.CODEC.decoder()
				.parse(JsonOps.INSTANCE, JsonParser.parseString("""
						{ "behavior": "metal", "young_gpa": 200, "density": 7850, "poisson": 0.3, "loss": 0.004,
						  "thickness": 0.05 }
						""")).getOrThrow();
		assertEquals(new MaterialDescriptor(SoundModel.MaterialKind.METAL, 200e9, 7850, 0.3, 0.004, 0.05, 0, 0.1, 0),
				fromJson, "fragments, restitution and shard have their defaults");
	}

	@Test
	void descriptorsAreCheckedWhereBuilt() {
		for (Runnable bad : new Runnable[] {
				() -> new MaterialDescriptor(null, 1e9, 1000, 0.3, 0.02, 0.1, 0, 0.1, 0),
				() -> new MaterialDescriptor(SoundModel.MaterialKind.WOOD, 1e6, 1000, 0.3, 0.02, 0.1, 0, 0.1, 0),
				() -> new MaterialDescriptor(SoundModel.MaterialKind.WOOD, 1e9, 1000, 0.6, 0.02, 0.1, 0, 0.1, 0),
				() -> new MaterialDescriptor(SoundModel.MaterialKind.WOOD, 1e9, 1000, 0.3, 0.02, 0.1, 300, 0.1, 0),
				() -> new MaterialDescriptor(SoundModel.MaterialKind.WOOD, 1e9, 1000, 0.3, 0.02, 0.1, 0, 0.99, 0),
				() -> new MaterialDescriptor(SoundModel.MaterialKind.WOOD, 1e9, 1000, 0.3, 0.02, 0.1, 0, 0.1, 0.5)}) {
			assertThrows(IllegalArgumentException.class, bad::run);
		}
	}

	@Test
	void theFloatsMatchTheNativesOrder() {
		float[] floats = MaterialDescriptor.glassy().toFloats();
		assertEquals(9, floats.length);
		assertEquals(SoundModel.MaterialKind.GLASS.ordinal(), (int) floats[0]);
		assertEquals(70e9f, floats[1], 1e6f);
		assertEquals(0.004f, floats[4], 1e-6f);
	}

	@Test
	void addonsKeepMaterialsToThemselves() {
		Identifier crystal = Identifier.fromNamespaceAndPath("mymod", "crystal");
		CustomMaterials.api("mymod").register(crystal, MaterialDescriptor.stony());
		assertTrue(CustomMaterials.isDefined(crystal));
		assertThrows(IllegalArgumentException.class,
				() -> CustomMaterials.api("othermod").register(Identifier.fromNamespaceAndPath("mymod", "clash"),
						MaterialDescriptor.stony()),
				"a material's namespace is its owner's");
		assertThrows(IllegalArgumentException.class, () -> CustomMaterials.api("mymod").register(crystal,
				MaterialDescriptor.stony()), "duplicate");
		Identifier foreign = Identifier.fromNamespaceAndPath("othermod", "theirs");
		assertEquals(null, CustomMaterials.layerName(foreign, 0), "an undefined material has no layer name");
		// (Mappings' own-material rule and block lookups need the game's registries; the gametest covers them.)
	}
}
