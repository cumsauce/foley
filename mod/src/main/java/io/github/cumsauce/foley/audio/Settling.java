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
	/** How long one buffer sounds, in milliseconds: the head delay plus the tick PARAMS asks the native code for. */
	public static final int LIFETIME_MS = DELAY_MS + Math.round(PARAMS[2] * 1000);
	/** The tail ramp off the tick, in milliseconds, so its hard cut can't click. */
	static final int FADE_MS = 1;
	/** Snow's gain is softer than gravel/sand's. */
	private static final float GAIN_DEFAULT = 0.05f;
	private static final float GAIN_SNOW = 0.03f;
	private final NativeBridge bridge;
	/** When the sounding buffer started, in {@link System#nanoTime()}. Client thread. */
	private long started;
	/** Whether a buffer has claimed the slot; the slot frees once its life has passed. Client thread. */
	private boolean live;

	public Settling(NativeBridge bridge) {
		this.bridge = bridge;
	}

	/** Whether {@code material} settles: granular shifts, earth and snow brushes; nothing else. */
	public static boolean shouldSettle(int material) {
		return material == Materials.GRANULAR || material == Materials.EARTH || material == Materials.SNOW;
	}

	/**
	 * Claims the one settle slot, or leaves it: false while a buffer started within its own LIFETIME_MS ms life is
	 * still sounding, so two contacts landing inside that window (a land and a step, a place and a step, in one tick)
	 * can't queue the identical deterministic grain over itself — a comb across the tick's 3 kHz band. Client thread.
	 */
	boolean due(long now) {
		if (live && now - started < LIFETIME_MS * 1_000_000L) return false;
		live = true;
		started = now;
		return true;
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
		if (!due(System.nanoTime())) return;
		short[] pcm = bridge.renderPrimitive(NativeBridge.PRIMITIVE_NOISE,
				new float[] {PARAMS[0], PARAMS[1], PARAMS[2], gain}, 0L);
		if (pcm == null || pcm.length == 0) return;
		fadeTail(pcm, FADE_MS * NativeBridge.SAMPLE_RATE / 1000);
		// Pad the head with silence so the tick sits DELAY_MS behind the impact.
		int delaySamples = DELAY_MS * NativeBridge.SAMPLE_RATE / 1000;
		short[] delayed = new short[pcm.length + delaySamples];
		System.arraycopy(pcm, 0, delayed, delaySamples, pcm.length);
		minecraft.getSoundManager().play(new BufferSound(Sounds.event("settle_gravel").location(),
				0.4f * FoleyClient.gain(), x, y, z, delayed));
	}

	/**
	 * Ramps the last {@code fadeSamples} of the grain down to silence in place, starting at full level: the native
	 * render ends on whatever sample the noise was at, and that hard cut into silence is a broadband click. Package
	 * for the test.
	 */
	static void fadeTail(short[] pcm, int fadeSamples) {
		if (pcm.length == 0) return;
		int n = Math.min(Math.max(fadeSamples, 2), pcm.length);
		for (int i = pcm.length - n; i < pcm.length; i++) {
			pcm[i] = (short) (pcm[i] * (pcm.length - 1 - i) / (n - 1));
		}
	}
}
