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
	/**
	 * Probes miss on a thin shoreline; keep the scene this many misses before falling silent —
	 * the dry-linger the cave and wind beds use, so a gap is a miss, not an end.
	 */
	private static final int MISSES_TO_STOP = 3;
	/** Wind speed, m/s, at which exposure saturates: a strong gale, {@link WindScene#STORM}. */
	private static final float GALE = 16f;

	private final NativeBridge bridge;
	private final RandomSource random = RandomSource.create();
	private @Nullable BedSound<SurfScene> sound;
	private SurfScene scene = SurfScene.SILENT;
	/** The shore last reported, re-tested rather than re-rolled, and how many probes have missed it. */
	private @Nullable BlockPos lastShore;
	private int misses;
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
		if (shore == null) {
			// A probe can miss a thin shoreline: keep the last scene for a few samples rather than
			// flapping the bed off and on again with a fresh seed.
			if (lastShore != null && holdsOnMiss(++misses)) return scene;
			lastShore = null;
			misses = 0;
			return SurfScene.SILENT;
		}
		// 1 at the water's edge, falling to 0 at the scan radius (horizontal, as findShore scans).
		double dx = shore.getX() + 0.5 - eye.x;
		double dz = shore.getZ() + 0.5 - eye.z;
		float near = Math.clamp(1f - (float) Math.hypot(dx, dz) / RADIUS, 0f, 1f);
		// Salt or fresh at the shore itself, not at the listener: a plains bank beside an ocean
		// is not a lake, and a pond beside a beach is not the sea.
		Holder<Biome> biome = level.getBiome(shore);
		boolean salt = biome.is(BiomeTags.IS_OCEAN) || biome.is(BiomeTags.IS_DEEP_OCEAN)
				|| biome.is(BiomeTags.IS_BEACH);
		// The storm the wind bed's sample mixes from: rain's weight to thunder's, clear to storm —
		// its own expression, not a copy that could drift from it.
		float storm = WindScene.storm(level);
		boolean stormy = level.isThundering() || level.isRaining();
		// The wind bed's reported wind, never a wind re-measured here; its m/s is scaled to
		// the 0..1 the bed takes, so calm and storm spread instead of saturating.
		var wind = FoleyClient.wind();
		float exposure = wind == null ? 0f : exposureFor(wind.scene().speed());
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
	 * The wind bed's reported speed, m/s, as the 0..1 exposure the bed takes: 0 in still air,
	 * 1 at {@value #GALE} m/s (a strong gale, {@link WindScene#STORM}'s reference) and clamped
	 * above it — gameplay wind rarely exceeds ~15 m/s, so the storm response spans the range
	 * instead of saturating at 1 m/s.
	 */
	public static float exposureFor(float windSpeed) {
		return Math.clamp(windSpeed / GALE, 0f, 1f);
	}

	/**
	 * Whether a probe that found no shore should hold the last scene rather than go silent.
	 * {@value #MISSES_TO_STOP} consecutive misses silence the bed: one random probe can miss a
	 * thin shoreline, and flapping the bed off and on restarts it with a fresh seed. Pure, so
	 * the latch can be pinned without a client.
	 */
	static boolean holdsOnMiss(int consecutiveMisses) {
		return consecutiveMisses < MISSES_TO_STOP;
	}

	/**
	 * Where open water meets land within {@code radius}: a column whose top is a water source with
	 * land a couple of blocks off one side, or null. Ported from {@link Life}'s one-shot scan; Life's
	 * copy goes with the one-shot it served. Columns whose chunk is not loaded — a teleport, respawn
	 * or login leaves chunks streaming in for a tick or two, and reading an unloaded one on the client
	 * throws — are skipped, never read.
	 */
	private @Nullable BlockPos findShore(ClientLevel level, Vec3 eye, int radius) {
		// Re-test the last shore before rolling again: the same column keeps `near` steady between
		// samples, instead of jumping to whichever column the next roll happens to hit.
		if (lastShore != null) {
			double lx = lastShore.getX() + 0.5 - eye.x;
			double lz = lastShore.getZ() + 0.5 - eye.z;
			if (Math.hypot(lx, lz) <= radius && isShore(level, lastShore.getX(), lastShore.getZ(), eye.y)) {
				misses = 0;
				return lastShore;
			}
		}
		for (int tries = 0; tries < 24; tries++) {
			int x = (int) Math.floor(eye.x) + random.nextInt(2 * radius + 1) - radius;
			int z = (int) Math.floor(eye.z) + random.nextInt(2 * radius + 1) - radius;
			if (!isShore(level, x, z, eye.y)) continue;
			lastShore = new BlockPos(x, level.getHeight(Heightmap.Types.MOTION_BLOCKING, x, z) - 1, z);
			misses = 0;
			return lastShore;
		}
		return null;
	}

	/**
	 * Whether the column at {@code (x, z)} is open water with land a couple of blocks off one side:
	 * the per-column test {@link #findShore} probes, and the re-test its remembered shore answers.
	 * A column whose chunk is not loaded — a teleport, respawn or login leaves chunks streaming in
	 * for a tick or two, and reading an unloaded one on the client throws — is never read.
	 */
	private boolean isShore(ClientLevel level, int x, int z, double eyeY) {
		if (!level.hasChunkAt(new BlockPos(x, (int) eyeY, z))) return false;
		int top = level.getHeight(Heightmap.Types.MOTION_BLOCKING, x, z) - 1;
		if (Math.abs(top - eyeY) > 16) return false;
		if (!level.getFluidState(new BlockPos(x, top, z)).isSourceOfType(Fluids.WATER)) return false;
		for (int[] d : new int[][] {{2, 0}, {-2, 0}, {0, 2}, {0, -2}}) {
			int landX = x + d[0];
			int landZ = z + d[1];
			if (!level.hasChunkAt(new BlockPos(landX, (int) eyeY, landZ))) continue;
			BlockPos land = new BlockPos(landX,
					level.getHeight(Heightmap.Types.MOTION_BLOCKING, landX, landZ) - 1, landZ);
			if (!level.getFluidState(land).getType().isSame(Fluids.WATER)
					&& Math.abs(land.getY() - top) <= 2) {
				return true;
			}
		}
		return false;
	}
}
