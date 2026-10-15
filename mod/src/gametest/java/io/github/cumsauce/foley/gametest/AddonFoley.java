package io.github.cumsauce.foley.gametest;

import io.github.cumsauce.foley.api.FoleyApi;
import io.github.cumsauce.foley.api.FoleyEntrypoint;
import io.github.cumsauce.foley.api.MaterialDescriptor;
import io.github.cumsauce.foley.api.SoundModel;
import io.github.cumsauce.foley.api.VoiceDescriptor;
import net.minecraft.resources.Identifier;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.SoundType;
import java.util.Map;

/**
 * The reference addon for the integration API (F0): the gametest mod declares this as its {@code foley} entrypoint
 * and registers sounds from code — two sound models, and its own material with the amethyst's sound group and block
 * mapped onto it (F1) — while {@code assets/foley-gametest/foley/sounds.json} defines two more from a pack.
 * {@link FoleyClientGameTest} plays them.
 */
public class AddonFoley implements FoleyEntrypoint {
	@Override
	public void onFoleyReady(FoleyApi api) {
		api.sounds().register(Identifier.fromNamespaceAndPath("foley-gametest", "code_drip"),
				new SoundModel.DripIntoWater(2.5f), 4);
		api.sounds().register(Identifier.fromNamespaceAndPath("foley-gametest", "code_step"),
				new SoundModel.Contact(SoundModel.ContactKind.STEP, SoundModel.MaterialKind.SNOW, 1f), 2);
		api.sounds().register(Identifier.fromNamespaceAndPath("foley-gametest", "code_chime"),
				new SoundModel.Tone(440f, 660f, 0.3f, 0.5f), 2);
		Identifier crystal = Identifier.fromNamespaceAndPath("foley-gametest", "crystal");
		api.materials().register(crystal,
				new MaterialDescriptor(SoundModel.MaterialKind.GLASS, 90e9, 3100, 0.2, 0.01, 0.05, 24, 0.5, 0.003));
		api.materials().mapSoundType(SoundType.AMETHYST, crystal);
		api.materials().mapBlock(Blocks.AMETHYST_BLOCK, crystal);
		EntityType<?> dolphin = net.minecraft.core.registries.BuiltInRegistries.ENTITY_TYPE
				.getValue(Identifier.withDefaultNamespace("dolphin"));
		api.voices().register(dolphin, new VoiceDescriptor(0.3, 300, 0.4, 0.6),
				Map.of("ambient", "ambient", "hurt", "hurt", "death", "death"));
	}
}
