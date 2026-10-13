package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.sounds.SoundSource;
import net.minecraft.tags.BlockTags;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * The player brushing through plants as they move: grass and flowers swish, leaves and bushes rustle, dry stalks
 * crackle ({@code foley:brush_<n>}), once every stride or so, louder the faster they go. With footsteps.
 */
public final class Brush {
	/** About a stride, blocks. */
	private static final double STRIDE = 0.7;

	private @Nullable Vec3 last;
	private double travelled;

	public void tick(Minecraft minecraft) {
		if (minecraft.level == null || minecraft.player == null
				|| !FoleyClient.wants(FoleyConfig::footsteps)) {
			last = null;
			return;
		}
		Vec3 now = minecraft.player.position();
		Vec3 before = last;
		last = now;
		if (before == null) return;
		double moved = Math.hypot(now.x - before.x, now.z - before.z) + 0.5 * Math.abs(now.y - before.y);
		if (moved < 0.01 || moved > 2) return;
		BlockPos feet = BlockPos.containing(now);
		int plant = plant(minecraft.level.getBlockState(feet));
		if (plant < 0) plant = plant(minecraft.level.getBlockState(feet.above()));
		if (plant < 0) {
			travelled = 0;
			return;
		}
		travelled += moved;
		if (travelled < STRIDE) return;
		travelled = 0;
		float speed = (float) (moved * 20);
		Sounds.play(minecraft, "brush_" + plant, SoundSource.PLAYERS, now.x, now.y + 0.5, now.z,
				Math.clamp(0.15f + 0.1f * speed, 0.15f, 0.7f), 0.9f + 0.2f * minecraft.player.getRandom().nextFloat());
	}

	/** 0 grass, 1 leaves, 2 dry stalks, or -1 for nothing to brush through. */
	static int plant(BlockState state) {
		if (state.is(BlockTags.LEAVES) || state.is(Blocks.SWEET_BERRY_BUSH) || state.is(Blocks.AZALEA)
				|| state.is(Blocks.FLOWERING_AZALEA) || state.is(Blocks.SUGAR_CANE) || state.is(Blocks.BUSH)) {
			return 1;
		}
		if (state.is(Blocks.DEAD_BUSH) || state.is(BlockTags.CROPS) || state.is(Blocks.SHORT_DRY_GRASS)
				|| state.is(Blocks.TALL_DRY_GRASS)) {
			return 2;
		}
		if (state.is(Blocks.SHORT_GRASS) || state.is(Blocks.TALL_GRASS) || state.is(Blocks.FERN)
				|| state.is(Blocks.LARGE_FERN) || state.is(BlockTags.FLOWERS)) {
			return 0;
		}
		return -1;
	}
}
