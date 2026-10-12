package io.github.cumsauce.foley.audio;

import net.minecraft.tags.BlockTags;
import net.minecraft.world.level.block.LeavesBlock;
import net.minecraft.world.level.block.SoundType;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.material.Fluids;

/** What rain hitting a block sounds like it lands on: the native {@code Surface} order. */
public final class Surfaces {
	public static final int WATER = 0;
	public static final int STONE = 1;
	public static final int LEAVES = 2;
	public static final int WOOD = 3;
	public static final int SOFT = 4;
	public static final int GLASS = 5;
	public static final int METAL = 6;

	private Surfaces() {}

	/** The surface {@code state} is, from its fluid, class, tags and sound. */
	public static int of(BlockState state) {
		if (state.getFluidState().getType().isSame(Fluids.WATER)) return WATER;
		if (state.getBlock() instanceof LeavesBlock || state.is(BlockTags.LEAVES)) return LEAVES;
		if (state.is(BlockTags.WOOL) || state.is(BlockTags.DIRT) || state.is(BlockTags.SAND)
				|| state.is(BlockTags.SNOW)) {
			return SOFT;
		}
		SoundType sound = state.getSoundType();
		if (sound == SoundType.GLASS || sound == SoundType.AMETHYST) return GLASS;
		if (sound == SoundType.METAL || sound == SoundType.COPPER || sound == SoundType.IRON
				|| sound == SoundType.ANVIL || sound == SoundType.NETHERITE_BLOCK) {
			return METAL;
		}
		if (sound == SoundType.WOOD || sound == SoundType.NETHER_WOOD || sound == SoundType.CHERRY_WOOD
				|| sound == SoundType.BAMBOO_WOOD || sound == SoundType.STEM) {
			return WOOD;
		}
		if (sound == SoundType.GRASS || sound == SoundType.GRAVEL || sound == SoundType.SAND
				|| sound == SoundType.WOOL || sound == SoundType.SNOW || sound == SoundType.MOSS
				|| sound == SoundType.ROOTED_DIRT || sound == SoundType.MUD) {
			return SOFT;
		}
		return STONE;
	}
}
