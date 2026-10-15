package io.github.cumsauce.foley.api;

import net.minecraft.resources.Identifier;
import net.minecraft.tags.TagKey;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.SoundType;

/**
 * A mod's own materials, as Foley's synthesis takes them: describe one ({@link MaterialDescriptor}), map blocks onto
 * it, and Foley layers its strikes, breaks and footsteps of those blocks with the descriptor's physics, under the
 * same settings toggles as its own.
 *
 * <p>Registration happens only while a {@link FoleyEntrypoint} runs. A block's material is looked up by its exact
 * block first, then its tags, then its {@link SoundType} (the game's own sound group), then Foley's ten built-ins.
 */
public interface FoleyMaterials {
	/**
	 * Defines {@code id} (in your mod's namespace) as {@code material}. Throws {@link IllegalStateException} after
	 * the entrypoint, {@link IllegalArgumentException} for a foreign namespace or a duplicate.
	 */
	void register(Identifier id, MaterialDescriptor material);

	/** Blocks of this exact {@code block} use {@code material}. Overrides everything but a more exact match. */
	void mapBlock(Block block, Identifier material);

	/** Every block in {@code tag} uses {@code material}. */
	void mapTag(TagKey<Block> tag, Identifier material);

	/**
	 * Blocks whose sound group is {@code type} use {@code material}, even where the block itself can't be asked
	 * (a break, whose block is already gone). A mod's own SoundType is the natural key.
	 */
	void mapSoundType(SoundType type, Identifier material);
}
