package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Holder;
import net.minecraft.sounds.SoundSource;
import net.minecraft.tags.BiomeTags;
import net.minecraft.util.RandomSource;
import net.minecraft.world.level.biome.Biome;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.level.material.Fluids;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * The surf bed (M17): one continuous shore bed at the ear, replacing the one-shot waves
 * {@link Life} used to fire every 6 to 12 s. Sampled every {@value #SAMPLE_EVERY} ticks: a
 * shore is found within {@value #RADIUS} blocks (Life's scan, ported), and the bed is told the
 * wave height (a lake's lapping, a calm sea, a rough one), how near the shore is, the storm,
 * and the wind bed's reported speed. A single bed, not a surround of four: the shore is one
 * broad source, and the 8-stream budget must not be spent four times over.
 */
public final class SurfController {
	private static final int SAMPLE_EVERY = 10;
	/** How far round to look for a shore, blocks: the one-shot's scan radius. */
	private static final int RADIUS = 32;

	private final NativeBridge bridge;
	private final RandomSource random = RandomSource.create();
	private @Nullable BedSound<SurfScene> sound;
	private SurfScene scene = SurfScene.SILENT;
	private int ticks;

	public SurfController(NativeBridge bridge) {
		this.bridge = bridge;
	}

	/** The current scene. */
	public SurfScene scene() {
		return scene;
	}

	/** The bed, if it's playing. */
	public @Nullable BedSound<SurfScene> sound() {
		return sound;
	}

	/** Ticks the surf bed. Client thread. */
	public void tick(Minecraft minecraft) {
		// The same toggle that gated the one-shot it replaces; no new config field.
		if (minecraft.level == null || minecraft.player == null
				|| !FoleyClient.wants(FoleyConfig::water)) {
			scene = SurfScene.SILENT;
			stop(minecraft);
			return;
		}
		if (ticks++ % SAMPLE_EVERY != 0) return;
		scene = sample(minecraft);
		if (scene.height() <= 0f) {
			stop(minecraft);
			return;
		}
		var ear = minecraft.player.getEyePosition();
		if (sound == null || !minecraft.getSoundManager().isActive(sound)) {
			// One broad source at the listener: the shore is heard as a field, not as a
			// point out of one ear, so it is placed rather than head-relative.
			sound = new BedSound<>("surf", SoundSource.AMBIENT, bridge, NativeBridge.BED_SURF, scene)
					.place(ear, false);
			minecraft.getSoundManager().play(sound);
		} else {
			sound.setScene(scene);
			sound.place(ear, false);
		}
	}

	private void stop(Minecraft minecraft) {
		if (sound != null) minecraft.getSoundManager().stop(sound);
		sound = null;
	}

	/** A fresh scene for the ear: the shore near it, the weather, and the wind bed's report. */
	private SurfScene sample(Minecraft minecraft) {
		var level = minecraft.level;
		var player = minecraft.player;
		if (level == null || player == null) return SurfScene.SILENT;
		Vec3 eye = player.getEyePosition();
		BlockPos shore = findShore(level, eye, RADIUS);
		if (shore == null) return SurfScene.SILENT;
		// 1 at the water's edge, falling to 0 at the scan radius (horizontal, as findShore scans).
		double dx = shore.getX() + 0.5 - eye.x;
		double dz = shore.getZ() + 0.5 - eye.z;
		float near = Math.clamp(1f - (float) Math.hypot(dx, dz) / RADIUS, 0f, 1f);
		Holder<Biome> biome = level.getBiome(BlockPos.containing(eye));
		boolean salt = biome.is(BiomeTags.IS_OCEAN) || biome.is(BiomeTags.IS_DEEP_OCEAN)
				|| biome.is(BiomeTags.IS_BEACH);
		// The storm the wind bed's sample mixes from: rain's weight to thunder's, clear to storm.
		float rain = level.getRainLevel(1f);
		float thunder = level.getThunderLevel(1f);
		float storm = Math.clamp(((WindScene.RAIN - WindScene.CLEAR) * rain
				+ (WindScene.STORM - WindScene.RAIN) * thunder) / (WindScene.STORM - WindScene.CLEAR), 0f, 1f);
		boolean stormy = level.isThundering() || level.isRaining();
		// The wind bed's reported wind, never a wind re-measured here.
		var wind = FoleyClient.wind();
		float exposure = wind == null ? 0f : wind.scene().speed();
		return new SurfScene(heightFor(salt, stormy), near, storm, exposure);
	}

	/**
	 * The wave height, metres: a lake's lapping, a calm sea, a rough one in a storm — the three
	 * tiers of {@link Life#SURF_M}.
	 */
	public static float heightFor(boolean salt, boolean stormy) {
		return !salt ? 0.3f : (stormy ? 2.2f : 1.0f);
	}

	/**
	 * How often a surge grain starts, grains per second: about 1/12 calm to 1/2 in a full storm
	 * with wind — the Java mirror of the native bed's interval, {@code 12 − 10·storm·exposure}
	 * seconds between surges.
	 */
	public static float surgeRate(float storm, float exposure) {
		float stormy = Math.clamp(storm * exposure, 0f, 1f);
		return 1f / (12f - 10f * stormy);
	}

	/**
	 * Where open water meets land within {@code radius}: a column whose top is a water source with
	 * land a couple of blocks off one side, or null. Ported from {@link Life}'s one-shot scan; Life's
	 * copy goes with the one-shot it served.
	 */
	private @Nullable BlockPos findShore(ClientLevel level, Vec3 eye, int radius) {
		for (int tries = 0; tries < 24; tries++) {
			int x = (int) Math.floor(eye.x) + random.nextInt(2 * radius + 1) - radius;
			int z = (int) Math.floor(eye.z) + random.nextInt(2 * radius + 1) - radius;
			int top = level.getHeight(Heightmap.Types.MOTION_BLOCKING, x, z) - 1;
			if (Math.abs(top - eye.y) > 16) continue;
			if (!level.getFluidState(new BlockPos(x, top, z)).isSourceOfType(Fluids.WATER)) continue;
			for (int[] d : new int[][] {{2, 0}, {-2, 0}, {0, 2}, {0, -2}}) {
				BlockPos land = new BlockPos(x + d[0],
						level.getHeight(Heightmap.Types.MOTION_BLOCKING, x + d[0], z + d[1]) - 1, z + d[1]);
				if (!level.getFluidState(land).getType().isSame(Fluids.WATER)
						&& Math.abs(land.getY() - top) <= 2) {
					return new BlockPos(x, top, z);
				}
			}
		}
		return null;
	}
}
