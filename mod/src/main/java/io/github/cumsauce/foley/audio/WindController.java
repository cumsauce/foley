package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.sounds.SoundSource;
import org.jspecify.annotations.Nullable;

/** Keeps Foley's wind blowing where there is any, following the scene four times a second. It layers on the game's own sounds. */
public final class WindController {
	private static final int SAMPLE_EVERY = 5;
	/** Below this, m/s, or this exposure, nothing to hear. */
	private static final float STILL = 0.5f;
	private static final float SHUT_IN = 0.02f;

	private final NativeBridge bridge;
	/** Heard all round, more from the open side. */
	private final SurroundBeds<WindScene> beds;
	private WindScene scene = WindScene.CALM;
	/** A speed set by command, overriding the weather, or null. */
	private @Nullable Float forced;
	private int ticks;

	public WindController(NativeBridge bridge) {
		this.bridge = bridge;
		this.beds = new SurroundBeds<>("wind", SoundSource.AMBIENT, bridge, NativeBridge.BED_WIND);
	}

	public WindScene scene() {
		return scene;
	}

	/** One of its beds, if it's playing. */
	public @Nullable BedSound<WindScene> sound() {
		return beds.any();
	}

	/** Forces the reference wind to {@code speed} m/s at the ear, or back to the weather with null. */
	public void force(@Nullable Float speed) {
		forced = speed;
		ticks = 0;
	}

	public void tick(Minecraft minecraft) {
		if (minecraft.level == null) {
			stop(minecraft);
			return;
		}
		if (ticks++ % SAMPLE_EVERY != 0) return;
		WindScene sampled = WindScene.sample(minecraft);
		if (forced != null) sampled = sampled.withSpeed(forced);
		// Among trees there's always a little air moving in the leaves: never quite still.
		if (sampled.leaves() > 0.3f && sampled.speed() < 1.2f) sampled = sampled.withSpeed(1.2f);
		scene = sampled;
		if (!io.github.cumsauce.foley.FoleyClient.wants(io.github.cumsauce.foley.config.FoleyConfig::wind)
				|| scene.speed() < STILL || (scene.exposure() < SHUT_IN && scene.leaves() < 0.05f)) {
			stop(minecraft);
			return;
		}
		beds.keep(minecraft, scene, scene.around());
	}

	private void stop(Minecraft minecraft) {
		beds.stop(minecraft);
	}
}
