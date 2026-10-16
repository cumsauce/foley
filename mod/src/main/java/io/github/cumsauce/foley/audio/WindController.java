package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.sounds.SoundSource;
import org.jspecify.annotations.Nullable;

/**
 * Keeps Foley's wind blowing where there is any, following the scene four times a second. It layers on the game's own sounds.
 *
 * The four quadrant gains reach each new sample by easing over seconds ({@link Pan}), biased toward a uniform field,
 * so the lean the nearby trees put in drifts like a weather vane instead of flapping like noise; and the play/still
 * gate carries hysteresis ({@link #heard}), so a listener hovering at the edge of still air or shelter doesn't hear
 * the bed cut out and back in.
 */
public final class WindController {
	private static final int SAMPLE_EVERY = 5;
	/** {@link #SAMPLE_EVERY} ticks at 20 TPS: the gap between samples, seconds. */
	private static final float SAMPLE_SECONDS = SAMPLE_EVERY / 20f;
	/** Below this, m/s, or this exposure, nothing to hear. */
	private static final float STILL = 0.5f;
	/** A running bed only gives up well below {@link #STILL}: hysteresis against chatter at the threshold. */
	private static final float STILL_STOP = STILL / 2;
	private static final float SHUT_IN = 0.02f;
	private static final float SHUT_IN_STOP = SHUT_IN / 2;
	/** Among leaves there's always a little air moving: this much is enough to hear, alone. */
	private static final float LEAVES = 0.05f;
	private static final float LEAVES_STOP = LEAVES / 2;

	private final NativeBridge bridge;
	/** Heard all round, more from the open side. */
	private final SurroundBeds<WindScene> beds;
	/** The quadrant gains on their way to the latest sample. */
	private final Pan pan = new Pan();
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
				|| !heard(beds.any() != null, scene.speed(), scene.exposure(), scene.leaves())) {
			stop(minecraft);
			return;
		}
		beds.keep(minecraft, scene, pan.step(scene.around()));
	}

	private void stop(Minecraft minecraft) {
		pan.reset();
		beds.stop(minecraft);
	}

	/**
	 * Whether the wind bed should be heard. As {@code CaveController.heard}: down, it starts only when the air is
	 * clearly audible (speed at least {@link #STILL}, or exposure {@link #SHUT_IN}, or leaves {@link #LEAVES}); up,
	 * it only gives up when the air is clearly out (speed below {@link #STILL_STOP}, exposure below
	 * {@link #SHUT_IN_STOP} and leaves below {@link #LEAVES_STOP}). A sample hovering in the band between changes
	 * nothing, so a threshold crossed by noise never flaps the bed.
	 */
	static boolean heard(boolean heard, float speed, float exposure, float leaves) {
		if (heard) return speed > STILL_STOP || exposure > SHUT_IN_STOP || leaves > LEAVES_STOP;
		return speed >= STILL || exposure >= SHUT_IN || leaves >= LEAVES;
	}

	/** Leans {@code was} a factor {@code alpha} of the way toward {@code target}: never past it. */
	static float ease(float was, float target, float alpha) {
		return was + (target - was) * alpha;
	}

	/** Leans {@code value} a factor {@code bias} of the way toward a uniform quarter of the field, 1/{@value SurroundBeds#AROUND}. */
	static float center(float value, float bias) {
		return value + (1f / SurroundBeds.AROUND - value) * bias;
	}

	/**
	 * The four quadrant gains the beds play at. Each new sample is biased {@value #BIAS} of the way toward uniform
	 * and the live gains ease toward it with time constant {@value #TAU} seconds (a 4 Hz sample's fraction
	 * 1 − e<sup>−0.25/{{@value #TAU}}</sup>): the tree-position lean the sample brings in arrives as a slow drift,
	 * never a snap, and no quarter runs away with the field. The first sample after a reset lands on its target
	 * outright, so a bed starting fresh is placed where the air is now, not where it was.
	 */
	static final class Pan {
		/** Seconds for most of the way there: the gap left is e<sup>−t/{{@value #TAU}}</sup>. */
		private static final float TAU = 4;
		/** How far each quarter leans toward equal, 0 to 1. */
		private static final float BIAS = 0.4f;

		/** The live gains, in {@link SurroundBeds}' quarter order. */
		private final float[] eased = new float[SurroundBeds.AROUND];
		private boolean ready;

		/** Eases the live gains toward {@code around} biased uniform; the first call lands on it outright. */
		float[] step(float[] around) {
			if (!ready) {
				for (int q = 0; q < eased.length; q++) eased[q] = center(around[q], BIAS);
				ready = true;
				return eased;
			}
			float alpha = 1 - (float) Math.exp(-SAMPLE_SECONDS / TAU);
			for (int q = 0; q < eased.length; q++) {
				eased[q] = ease(eased[q], center(around[q], BIAS), alpha);
			}
			return eased;
		}

		/** Forgets the gains, so the next sample lands on its own rather than leaning from the old place. */
		void reset() {
			ready = false;
		}
	}
}
