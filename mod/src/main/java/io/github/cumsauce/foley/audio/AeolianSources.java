package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.natives.NativeBridge;
import java.util.HashSet;
import java.util.Set;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.tags.BlockTags;
import net.minecraft.tags.TagKey;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;

/**
 * Aeolian tones (M18): Strouhal whistles off iron bars, fences, lanterns, bells and bamboo within
 * {@value #RADIUS} blocks of the listener, silent below {@value #STILL} m/s. Runs every 10 ticks and
 * plays one positional whistle per category (iron bars, fences, lanterns, bells, bamboo) at the
 * nearest qualifying block. Each whistle's pitch is the Strouhal frequency {@code f ≈ 0.2·U/d} where
 * {@code d} is the obstacle's characteristic diameter and {@code U} is the wind speed.
 */
public final class AeolianSources {
	private static final int SAMPLE_EVERY = 10;
	/** How far round to look for qualifying blocks, blocks. */
	private static final int RADIUS = 24;
	/** Iron bars (typical picket, ~40 mm wide). */
	private static final float D_IRON_BARS = 0.040f;
	/** Fence rails (typical picket, ~25 mm). */
	private static final float D_FENCE = 0.025f;
	/** Lantern finial, ~80 mm. */
	private static final float D_LANTERN = 0.080f;
	/** Bell rim, ~150 mm. */
	private static final float D_BELL = 0.150f;
	/** Bamboo internode, ~50 mm. */
	private static final float D_BAMBOO = 0.050f;
	/** Seconds per whistle. */
	private static final float SECONDS = 0.5f;
	/** Per-whistle gain. */
	private static final float GAIN = 0.6f;
	/** Threshold below which the wind is too quiet for aeolian tones (m/s). */
	private static final float STILL = 2.0f;
	/** How far above the heightmap's top block we still scan for source blocks. */
	private static final int LOOK_ABOVE = 6;

	private final NativeBridge bridge;
	private int played;
	private int ticks;

	public AeolianSources(NativeBridge bridge) {
		this.bridge = bridge;
	}

	public int played() {
		return played;
	}

	/** Below this wind speed, the air is too quiet for aeolian tones. */
	static float silenceThreshold() {
		return STILL;
	}

	/** The Strouhal pitch (Hz) for an obstacle of characteristic diameter {@code d} (m) at wind speed {@code u} (m/s). */
	static float pitch(float d, float u) {
		return 0.2f * u / d;
	}

	public void tick(Minecraft minecraft) {
		if (minecraft.level == null || minecraft.player == null) return;
		if (!FoleyClient.wants(FoleyConfig::aeolian)) return;
		if (ticks++ % SAMPLE_EVERY != 0) return;
		var wind = FoleyClient.wind();
		if (wind == null) return;
		float u = wind.scene().speed();
		if (u < STILL) return;
		Vec3 eye = minecraft.player.getEyePosition();
		Set<String> fired = new HashSet<>();
		findNearest(minecraft, eye, state -> state.is(Blocks.IRON_BARS), "iron_bars").ifPresent(p -> {
			if (fired.add("iron_bars")) play(minecraft, p, D_IRON_BARS, u);
		});
		findNearestByTag(minecraft, eye, BlockTags.FENCES, "fence").ifPresent(p -> {
			if (fired.add("fence")) play(minecraft, p, D_FENCE, u);
		});
		findNearest(minecraft, eye, state -> state.is(Blocks.LANTERN) || state.is(Blocks.SOUL_LANTERN), "lantern")
				.ifPresent(p -> {
					if (fired.add("lantern")) play(minecraft, p, D_LANTERN, u);
				});
		findNearest(minecraft, eye, state -> state.is(Blocks.BELL), "bell").ifPresent(p -> {
			if (fired.add("bell")) play(minecraft, p, D_BELL, u);
		});
		findNearestByTag(minecraft, eye, BlockTags.BAMBOO_BLOCKS, "bamboo").ifPresent(p -> {
			if (fired.add("bamboo")) play(minecraft, p, D_BAMBOO, u);
		});
	}

	/** The nearest qualifying block within {@link #RADIUS} blocks of {@code eye}, scanned per column above the
	 * heightmap's top. */
	private java.util.Optional<BlockPos> findNearest(Minecraft minecraft, Vec3 eye,
			java.util.function.Predicate<BlockState> match, String key) {
		return findNearestBy(minecraft, eye, match);
	}

	private java.util.Optional<BlockPos> findNearestByTag(Minecraft minecraft, Vec3 eye, TagKey<Block> tag, String key) {
		return findNearestBy(minecraft, eye, state -> state.is(tag));
	}

	private java.util.Optional<BlockPos> findNearestBy(Minecraft minecraft, Vec3 eye,
			java.util.function.Predicate<BlockState> match) {
		BlockPos.MutableBlockPos pos = new BlockPos.MutableBlockPos();
		BlockPos nearest = null;
		double best = RADIUS * RADIUS;
		int ex = (int) Math.floor(eye.x), ez = (int) Math.floor(eye.z);
		int minY = minecraft.level.getMinY();
		int maxY = minecraft.level.getMaxY();
		for (int dx = -RADIUS; dx <= RADIUS; dx++) {
			for (int dz = -RADIUS; dz <= RADIUS; dz++) {
				int x = ex + dx, z = ez + dz;
				int top = minecraft.level.getHeight(Heightmap.Types.MOTION_BLOCKING, x, z);
				// Look at the topmost blocks where fences/bars/bells/lanterns/bamboo sit, plus a few blocks
				// above in case the heightmap undercounts leaves or snow.
				int from = Math.max(minY, top - 4);
				int to = Math.min(maxY, top + LOOK_ABOVE);
				for (int y = from; y <= to; y++) {
					pos.set(x, y, z);
					if (!minecraft.level.isLoaded(pos)) continue;
					if (!match.test(minecraft.level.getBlockState(pos))) continue;
					double d2 = eye.distanceToSqr(x + 0.5, y + 0.5, z + 0.5);
					if (d2 < best) {
						best = d2;
						nearest = pos.immutable();
					}
				}
			}
		}
		return java.util.Optional.ofNullable(nearest);
	}

	private void play(Minecraft minecraft, BlockPos at, float d, float u) {
		if (!FoleyClient.oneShot()) return;
		Vec3 eye = minecraft.player.getEyePosition();
		if (eye.distanceToSqr(at.getX() + 0.5, at.getY() + 0.5, at.getZ() + 0.5) > RADIUS * RADIUS) return;
		float[] params = {d, u, SECONDS, GAIN};
		long seed = at.asLong() ^ 0xAE11AE11BEEFL;
		try {
			short[] pcm = bridge.renderPrimitive(NativeBridge.PRIMITIVE_AEOLIAN, params, seed);
			BufferSound sound = new BufferSound(Sounds.event("aeolian").location(), FoleyClient.gain(),
					at.getX() + 0.5, at.getY() + 0.5, at.getZ() + 0.5, pcm);
			minecraft.getSoundManager().play(sound);
			played++;
		} catch (Exception e) {
			// Native render failed; skip this scan iteration.
		}
	}
}