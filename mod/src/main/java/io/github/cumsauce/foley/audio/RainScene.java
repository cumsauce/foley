package io.github.cumsauce.foley.audio;

import java.util.Arrays;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.biome.Biome;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

/**
 * The rain round the listener, sampled from the world: how hard it falls (from the game's rain and thunder levels,
 * about 10 mm/h at full rain and 50 in a thunderstorm), what it lands on (the top block of each column within
 * {@value #RADIUS} blocks where it rains, nearer ones counting more), and what the listener shelters under. Snow and
 * dry biomes don't patter. With nothing open to the sky near the listener (deep in a cave), rain isn't heard.
 *
 * @param rate     mm/h
 * @param surfaces the share of each {@link Surfaces} kind
 * @param roof     the surface overhead, or -1 under the sky
 * @param height   how far above the ear the roof is, in blocks
 */
public record RainScene(float rate, float[] surfaces, int roof, float height) {
	public static final RainScene DRY = new RainScene(0, new float[] {0, 0, 0, 0, 1, 0, 0}, -1, 0);
	static final int RADIUS = 8;
	private static final int STEP = 2;
	/** A roof more than this far up isn't drummed on loud enough to tell; past it, rain is far away. */
	private static final int ROOF_REACH = 16;

	public static RainScene sample(Minecraft minecraft) {
		ClientLevel level = minecraft.level;
		if (level == null || minecraft.player == null) return DRY;
		float raining = level.getRainLevel(1f);
		if (raining <= 0.01f) return DRY;
		float rate = raining * (10 + 40 * level.getThunderLevel(1f));
		Vec3 eye = minecraft.player.getEyePosition();
		float[] surfaces = new float[7];
		float open = 0;
		BlockPos.MutableBlockPos pos = new BlockPos.MutableBlockPos();
		for (int dx = -RADIUS; dx <= RADIUS; dx += STEP) {
			for (int dz = -RADIUS; dz <= RADIUS; dz += STEP) {
				int x = (int) Math.floor(eye.x) + dx;
				int z = (int) Math.floor(eye.z) + dz;
				int top = level.getHeight(Heightmap.Types.MOTION_BLOCKING, x, z);
				pos.set(x, top, z);
				if (level.getBiome(pos).value().getPrecipitationAt(pos, level.getSeaLevel()) != Biome.Precipitation.RAIN) {
					continue;
				}
				float weight = 1f / (1 + dx * dx + dz * dz);
				if (top <= eye.y + 1) open += weight;
				surfaces[Surfaces.of(level.getBlockState(pos.setY(top - 1)))] += weight;
			}
		}
		float total = 0;
		for (float share : surfaces) total += share;
		if (total <= 0) return DRY;
		for (int i = 0; i < surfaces.length; i++) surfaces[i] /= total;
		// Under a roof: the first solid block above the ear, if close enough to drum on.
		int roof = -1;
		float height = 0;
		int column = level.getHeight(Heightmap.Types.MOTION_BLOCKING, (int) Math.floor(eye.x), (int) Math.floor(eye.z));
		if (column > eye.y + 1) {
			for (int up = 1; up <= ROOF_REACH; up++) {
				pos.set(Math.floor(eye.x), Math.floor(eye.y) + up, Math.floor(eye.z));
				var state = level.getBlockState(pos);
				if (state.isSolid()) {
					roof = Surfaces.of(state);
					height = (float) (pos.getY() - eye.y);
					break;
				}
			}
			// Deep inside, out of the rain and away from any opening: nothing to hear.
			if (roof == -1 && open <= 0) return DRY;
		}
		return new RainScene(rate, surfaces, roof, height);
	}

	@Override
	public String toString() {
		return String.format(java.util.Locale.ROOT, "RainScene[rate=%.1f mm/h, surfaces=%s, roof=%d at %.1f]", rate,
				Arrays.toString(surfaces), roof, height);
	}
}
