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
 * eases in and surfacing eases out without a click. The bed is silent above sea
 * level (the controller sets level = 0). The fade on emerging is built into
 * the bed's level glide.
 *
 * <p>Head-in-water is determined by the fluid at the ear position: only water
 * (not lava, not air, not any other fluid) puts the listener under. The native
 * SUBMERGED flag — which low-passes every other Foley bed at 1 kHz — is latched
 * with hysteresis (it holds through brief dry ticks so a wave cannot flap it)
 * and is only raised while Foley's underwater ambience is wanted by the config.
 */
public final class UnderwaterController {
	/** The depth (in blocks below sea level) at which the bed's level reaches 1. */
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
	 * Samples the world: the listener's depth below sea level (clamped to
	 * {@value #MAX_DEPTH}), scaled to a 0..=1 level. The head must be in water for any of
	 * it — that wetness also latches the native submerged flag. Sea level is 62 in
	 * the Overworld (the vanilla default) — read from the dimension so the Nether
	 * and other dimensions stay silent.
	 */
	private UnderwaterScene sample(Minecraft minecraft) {
		ClientLevel level = minecraft.level;
		var player = minecraft.player;
		var eye = player.getEyePosition();
		// Head-in-water: the fluid at the ear is water. (We don't gate on
		// being able to breathe: surfacing into a wave still muffles for a
		// frame.)
		var fluid = level.getFluidState(BlockPos.containing(eye));
		wet = fluid.getType().isSame(Fluids.WATER);
		if (!wet) return UnderwaterScene.DRY;
		float sea = level.getSeaLevel();
		float depth = Math.max(0f, sea - (float) eye.y);
		float amount = Math.min(1f, depth / MAX_DEPTH);
		return new UnderwaterScene(amount);
	}
}
