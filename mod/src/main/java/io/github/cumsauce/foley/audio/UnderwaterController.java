package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.core.BlockPos;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.level.material.Fluids;
import org.jspecify.annotations.Nullable;

/**
 * M14: keeps Foley's underwater bed playing when the player's head is under
 * water. A single {@link BedSound} holds the scene — its level glides from 0 to 1
 * over about a second on submerging, and back on emerging, so a sudden dive
 * eases in and surfacing eases out without a click. The bed is silent while the
 * ear is out of the water (the controller returns level 0), and its level is how
 * deep the ear sits under the water surface. The fade on emerging is built into
 * the bed's level glide.
 *
 * <p>Head-in-water is determined by the fluid at the ear position: only water
 * (not lava, not air, not any other fluid) puts the listener under. The native
 * SUBMERGED flag — which low-passes every other Foley bed at 1 kHz — is latched
 * with hysteresis (it holds through brief dry ticks so a wave cannot flap it)
 * and is only raised while Foley's underwater ambience is wanted by the config.
 */
public final class UnderwaterController {
	/** The depth (in blocks under the water surface) at which the bed's level reaches 1. */
	private static final float MAX_DEPTH = 30f;
	/** Ticks the submerged flag holds after the ear leaves water, so wave troughs don't flap the low-pass: the flag lets go on the {@link #DRY_LINGER}th dry tick. */
	static final int DRY_LINGER = 20;
	/** Ticks per sample. Two ticks a second matches {@link PlacedBeds} and {@link WindController}. */
	private static final int SAMPLE_EVERY = 10;

	private final NativeBridge bridge;
	private UnderwaterScene scene = UnderwaterScene.DRY;
	/** Whether the fluid at the ear is water, sampled straight from the world. */
	private boolean wet;
	/** The latched submerged flag handed to the native side: wet, or not yet dry for {@link #DRY_LINGER} ticks. */
	private boolean submerged;
	/** Ticks since the ear last touched water. */
	private int dryFor;
	private @Nullable BedSound<UnderwaterScene> sound;
	/** Ticks since the bed last heard dry: stop fading it 1 s after the listener emerged. */
	private int fading;

	public UnderwaterController(NativeBridge bridge) {
		this.bridge = bridge;
	}

	/** The bed's current scene. */
	public UnderwaterScene scene() {
		return scene;
	}

	/** The bed, if it's playing. */
	public @Nullable BedSound<UnderwaterScene> sound() {
		return sound;
	}

	public void tick(Minecraft minecraft) {
		if (minecraft.level == null || minecraft.player == null) {
			scene = UnderwaterScene.DRY;
			wet = false;
			dryFor = 0;
			submerged = false;
			stop(minecraft);
			bridge.setSubmerged(false);
			return;
		}
		scene = sample(minecraft);
		dryFor = dryTicks(wet, dryFor);
		submerged = latchedSubmerged(submerged, wet, dryFor);
		boolean wanted = FoleyClient.wants(FoleyConfig::underwater);
		bridge.setSubmerged(submergedFlag(wanted, submerged));
		if (!wanted || scene.level() <= 0f) {
			stop(minecraft);
			return;
		}
		keep(minecraft);
	}

	/**
	 * The native SUBMERGED flag: only while Foley's underwater ambience is wanted <em>and</em> the listener
	 * is (latched as) submerged. With the underwater setting off — or Foley itself bypassed — a wet ear
	 * must not low-pass every other Foley bed at 1 kHz.
	 */
	static boolean submergedFlag(boolean underwaterWanted, boolean submergedNow) {
		return underwaterWanted && submergedNow;
	}

	/**
	 * Ticks since the ear last touched water: zero while wet, counting up while dry and saturating
	 * at {@link #DRY_LINGER} — the tick at which {@link #latchedSubmerged} lets go.
	 */
	static int dryTicks(boolean wet, int dryFor) {
		return wet ? 0 : Math.min(dryFor + 1, DRY_LINGER);
	}

	/**
	 * Hysteresis for the submerged flag: it latches the moment the ear is in water and only lets go
	 * once it has been dry for {@link #DRY_LINGER} ticks — on the 20th dry tick, matching the
	 * constant's documented meaning. Bobbing at the surface — wet one tick, dry the next — must not
	 * flap the flag (and with it every other bed's low-pass) sample by sample.
	 */
	static boolean latchedSubmerged(boolean wasSubmerged, boolean wet, int dryFor) {
		if (wet) return true;
		return wasSubmerged && dryFor < DRY_LINGER;
	}

	private void keep(Minecraft minecraft) {
		fading = 0;
		if (sound != null && minecraft.getSoundManager().isActive(sound)) {
			sound.setScene(scene);
			return;
		}
		sound = new BedSound<>("underwater", SoundSource.AMBIENT, bridge, NativeBridge.BED_UNDERWATER, scene)
				.follow(io.github.cumsauce.foley.audio.SurroundBeds.offset(0), 1f);
		minecraft.getSoundManager().play(sound);
	}

	private void stop(Minecraft minecraft) {
		if (sound == null) return;
		sound.weigh(0);
		fading += SAMPLE_EVERY;
		if (fading > 20) {
			minecraft.getSoundManager().stop(sound);
			sound = null;
			fading = 0;
		}
	}

	/**
	 * Samples the world: the listener's depth under the water surface above their ear (clamped to
	 * {@value #MAX_DEPTH}), scaled to a 0..=1 level. The head must be in water for any of it — that
	 * wetness also latches the native submerged flag. The surface is the first dry block above the
	 * ear's own, not the world's sea level: a superflat's sea sits under its bedrock and a mountain
	 * lake's far below its own surface, so sea level read "dry" for swimmers in both.
	 */
	private UnderwaterScene sample(Minecraft minecraft) {
		ClientLevel level = minecraft.level;
		var player = minecraft.player;
		var eye = player.getEyePosition();
		// Head-in-water: the fluid at the ear is water. (We don't gate on
		// being able to breathe: surfacing into a wave still muffles for a
		// frame.)
		var at = BlockPos.containing(eye);
		var fluid = level.getFluidState(at);
		wet = fluid.getType().isSame(Fluids.WATER);
		if (!wet) return UnderwaterScene.DRY;
		return new UnderwaterScene(depthLevel(surfaceAbove(level, at), eye.y));
	}

	/**
	 * The Y of the water surface above the ear's block: the first block that is not water, scanning
	 * up from just over the ear. Capped a hair past {@link #MAX_DEPTH} — deeper than that can only
	 * mean a full level, and the cap keeps the scan short.
	 */
	private static int surfaceAbove(ClientLevel level, BlockPos at) {
		int x = at.getX(), z = at.getZ();
		int y = at.getY() + 1;
		for (int steps = 0; steps <= (int) MAX_DEPTH + 2; steps++, y++) {
			if (!level.getFluidState(new BlockPos(x, y, z)).getType().isSame(Fluids.WATER)) return y;
		}
		return y;
	}

	/**
	 * How deep the ear sits under the water surface: 0 at or above it, 1 at {@link #MAX_DEPTH} blocks
	 * under. Measured from the surface, never from the world's sea level: a superflat's sea is under
	 * its bedrock and a mountain lake's is below its own surface, so sea level would read "dry" for
	 * swimmers in both — the bed played nowhere but the vanilla ocean.
	 */
	static float depthLevel(double surfaceY, double eyeY) {
		float depth = (float) Math.max(0d, surfaceY - eyeY);
		return Math.min(1f, depth / MAX_DEPTH);
	}
}
