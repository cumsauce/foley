package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.levelgen.Heightmap;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * Keeps Foley's cave bed playing underground, sampling depth, stone mass, openness and drips
 * twice a second. The bed is a single instance: only one running at a time.
 */
public final class CaveController {
	private static final int SAMPLE_EVERY = 10;
	/** Below this openness, the bed is heard. Above 0.2 the bed fades out (matches the old exposure threshold). */
	private static final float SHUT_IN = 0.2f;
	/** A running bed stops only at this wider openness: the hysteresis band so threshold chatter can't flap it. */
	private static final float SHUT_IN_STOP = SHUT_IN + 0.08f;
	/** How many blocks below the surface the ear has to be before the bed turns on. */
	private static final int MIN_DEPTH = 8;
	/** A running bed stops only shallower than this: the hysteresis band for the depth threshold. */
	private static final int MIN_DEPTH_STOP = MIN_DEPTH - 2;
	/** A column scan: how many blocks above the ear to look for stone mass. */
	private static final int COLUMN_TOP = 60;
	/** The depth in blocks a full-depth scene is measured against; {@code CaveScene.depth} is blocks divided by this. */
	private static final float DEPTH_FULL = 60f;

	private final NativeBridge bridge;
	private @Nullable BedSound<CaveScene> sound;
	private CaveScene scene = CaveScene.SILENT;
	/** Whether the cave bed currently wants to be heard — hysteresis state (see {@link #heard}). */
	private boolean heard;
	private int ticks;

	public CaveController(NativeBridge bridge) {
		this.bridge = bridge;
	}

	public CaveScene scene() {
		return scene;
	}

	public @Nullable BedSound<CaveScene> sound() {
		return sound;
	}

	public void tick(Minecraft minecraft) {
		if (minecraft.level == null || minecraft.player == null) {
			heard = false;
			stop(minecraft);
			return;
		}
		if (!FoleyClient.wants(FoleyConfig::caveAtmosphere)) {
			heard = false;
			stop(minecraft);
			return;
		}
		if (ticks++ % SAMPLE_EVERY != 0) {
			// Between samples, keep the bed following the new scene.
			if (sound != null) sound.setScene(scene);
			return;
		}
		scene = sample(minecraft);
		// The scene carries normalised depth; the hysteresis thresholds are in blocks, so undo the scale here.
		heard = heard(heard, scene);
		if (!heard) {
			stop(minecraft);
			return;
		}
		if (sound == null || !minecraft.getSoundManager().isActive(sound)) {
			sound = new BedSound<>("cave", SoundSource.AMBIENT, bridge, NativeBridge.BED_CAVE, scene)
					.place(minecraft.player.getEyePosition(), false);
			minecraft.getSoundManager().play(sound);
		} else {
			sound.setScene(scene);
		}
	}

	/**
	 * Hysteresis for "the cave bed is heard": it turns on only clearly shut in (depth at least
	 * {@link #MIN_DEPTH}, openness at most {@link #SHUT_IN}) and turns off only clearly out (shallower
	 * than {@link #MIN_DEPTH_STOP} or more open than {@link #SHUT_IN_STOP}) — a depth or openness
	 * hovering around a single threshold must not flap the bed on and off.
	 */
	static boolean heard(boolean wasHeard, float depth, float openness) {
		if (wasHeard) return depth >= MIN_DEPTH_STOP && openness <= SHUT_IN_STOP;
		return depth >= MIN_DEPTH && openness <= SHUT_IN;
	}

	/**
	 * The same verdict for a sampled scene: {@code scene.depth()} is normalised (blocks over
	 * {@link #DEPTH_FULL}), while the thresholds above are in blocks. This is the call site that
	 * couples the two, and the regression it guards is passing the normalised value straight in —
	 * a depth of 0.85 then reads as 0.85 of a block and the bed never plays at all.
	 */
	static boolean heard(boolean wasHeard, CaveScene scene) {
		return heard(wasHeard, scene.depth() * DEPTH_FULL, scene.openness());
	}

	private void stop(Minecraft minecraft) {
		if (sound != null) minecraft.getSoundManager().stop(sound);
		sound = null;
	}

	/** A fresh scene for the ear at the player's eye. */
	static CaveScene sample(Minecraft minecraft) {
		var player = minecraft.player;
		Level level = minecraft.level;
		if (player == null || level == null) return CaveScene.SILENT;
		int x = (int) Math.floor(player.getX());
		int y = (int) Math.floor(player.getEyeY());
		int z = (int) Math.floor(player.getZ());
		int surface = level.getHeight(Heightmap.Types.WORLD_SURFACE, x, z);
		int depth = Math.max(0, surface - y);
		int mass = 0;
		int drips = 0;
		int scanTop = Math.min(surface, y + COLUMN_TOP);
		for (int dy = y; dy < scanTop; dy++) {
			BlockPos pos = new BlockPos(x, dy, z);
			BlockState state = level.getBlockState(pos);
			if (!state.isAir()) mass++;
			if (isDripSource(state)) drips++;
		}
		var wind = FoleyClient.wind();
		float openness = wind == null ? 0 : wind.scene().exposure();
		float gain = Math.min(1, depth / 40f) * Math.min(1, mass / 30f);
		return new CaveScene(
				Math.min(1, depth / DEPTH_FULL),
				Math.min(1, mass / 60f),
				openness,
				Math.min(1, drips / 8f),
				gain);
	}

	/** A block whose presence counts as a drip source: dripstone (pointed down) or any waterlogged block. */
	private static boolean isDripSource(BlockState state) {
		if (state.is(Blocks.POINTED_DRIPSTONE)) {
			// Dripstone dripping is only the stalactites (pointing down).
			if (state.hasProperty(BlockStateProperties.VERTICAL_DIRECTION)
					&& state.getValue(BlockStateProperties.VERTICAL_DIRECTION) == Direction.DOWN) {
				return true;
			}
		}
		if (state.hasProperty(BlockStateProperties.WATERLOGGED)
				&& state.getValue(BlockStateProperties.WATERLOGGED)) {
			return true;
		}
		return false;
	}
}
