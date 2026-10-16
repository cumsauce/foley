package io.github.cumsauce.foley.audio;

import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.chunk.LevelChunk;
import net.minecraft.world.level.chunk.LevelChunkSection;

/** Counts mangrove leaves within a radius (M19: mangrove drone). */
public final class MangroveLeaves {
	private MangroveLeaves() {}

	/**
	 * The mangrove leaves within {@code radius} blocks horizontally and 8 blocks up
	 * or down of {@code center}, capped at 200.
	 */
	public static int count(ClientLevel level, BlockPos center, int radius) {
		int count = 0;
		int cx0 = (center.getX() - radius) >> 4;
		int cx1 = (center.getX() + radius) >> 4;
		int cz0 = (center.getZ() - radius) >> 4;
		int cz1 = (center.getZ() + radius) >> 4;
		int yMin = Math.max(level.getMinY(), center.getY() - 8);
		int yMax = Math.min(level.getMaxY(), center.getY() + 8);
		for (int cx = cx0; cx <= cx1; cx++) {
			for (int cz = cz0; cz <= cz1; cz++) {
				LevelChunk chunk = level.getChunk(cx, cz);
				for (int sy = yMin >> 4; sy <= yMax >> 4; sy++) {
					LevelChunkSection section = chunk.getSection(level.getSectionIndexFromSectionY(sy));
					if (section.hasOnlyAir()) continue;
					int x0 = Math.max(cx << 4, center.getX() - radius);
					int x1 = Math.min((cx << 4) + 15, center.getX() + radius);
					int z0 = Math.max(cz << 4, center.getZ() - radius);
					int z1 = Math.min((cz << 4) + 15, center.getZ() + radius);
					int y0 = Math.max(sy << 4, yMin);
					int y1 = Math.min((sy << 4) + 15, yMax);
					for (int x = x0; x <= x1; x++) {
						for (int y = y0; y <= y1; y++) {
							for (int z = z0; z <= z1; z++) {
								if (count >= 200) return 200;
								if (section.getBlockState(x & 15, y & 15, z & 15).is(Blocks.MANGROVE_LEAVES)) {
									count++;
								}
							}
						}
					}
				}
			}
		}
		return count;
	}
}
