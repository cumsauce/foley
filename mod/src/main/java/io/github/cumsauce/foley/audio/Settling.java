package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.sounds.SoundSource;

/**
 * A 20 ms bandpassed noise tick played 18 ms after footsteps or block placement on gravel, sand or snow. {@link
 * Layers#layer} calls {@link #settle} once it has resolved the material; the tick goes through the same
 * {@link BufferSound} path the bridge already drives for the shulker open, padded at the head with silence so the
 * tail of the M4 impact lands in front of it.
 */
public final class Settling {
	/** The delay after the impact, in milliseconds (in the spec's 10 to 30 ms window). */
	public static final int DELAY_MS = 18;
	/** The settle primitive's parameters: centre 3000 Hz, q 1.5, seconds 0.02, gain 0.05. */
	public static final float[] PARAMS = {3000.0f, 1.5f, 0.02f, 0.05f};
	/** Snow's gain is softer than gravel/sand's. */
	private static final float GAIN_DEFAULT = 0.05f;
	private static final float GAIN_SNOW = 0.03f;
	private final NativeBridge bridge;

	public Settling(NativeBridge bridge) {
		this.bridge = bridge;
	}

	/** Whether {@code material} settles: granular shifts, earth and snow brushes; nothing else. */
	public static boolean shouldSettle(int material) {
		return material == Materials.GRANULAR || material == Materials.EARTH || material == Materials.SNOW;
	}

	/** Plays the settle tick at {@code (x, y, z)} for {@code material}, if Foley wants settling. */
	public void settle(Minecraft minecraft, double x, double y, double z, int material) {
		if (bridge == null) return;
		if (!FoleyClient.wants(FoleyConfig::settling)) return;
		if (!shouldSettle(material)) return;
		float gain = material == Materials.SNOW ? GAIN_SNOW : GAIN_DEFAULT;
		playSettle(minecraft, x, y, z, gain);
	}

	private void playSettle(Minecraft minecraft, double x, double y, double z, float gain) {
		short[] pcm = bridge.renderPrimitive(NativeBridge.PRIMITIVE_NOISE,
				new float[] {PARAMS[0], PARAMS[1], PARAMS[2], gain}, 0L);
		if (pcm == null || pcm.length == 0) return;
		// Pad the head with silence so the tick sits DELAY_MS behind the impact.
		int delaySamples = DELAY_MS * NativeBridge.SAMPLE_RATE / 1000;
		short[] delayed = new short[pcm.length + delaySamples];
		System.arraycopy(pcm, 0, delayed, delaySamples, pcm.length);
		minecraft.getSoundManager().play(new BufferSound(Sounds.event("settle_gravel").location(),
				0.4f * FoleyClient.gain(), x, y, z, delayed));
	}
}