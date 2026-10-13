package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.sounds.SoundSource;
import org.jspecify.annotations.Nullable;

/**
 * Keeps Foley's rain going while it rains where the listener can hear it, following the scene twice a second, and says
 * when the game's own rain sounds should stay quiet (they are replaced, not layered).
 */
public final class RainController {
	private static final int SAMPLE_EVERY = 10;
	/** The rain rate heard now, mm/h, following the sky's slowly. */
	private float heardRate;
	/** Rain stops this many ticks after the scene goes dry, so a gap in the clouds doesn't cut it off. */
	private static final int LINGER = 100;

	private final NativeBridge bridge;
	/** Heard all round, more from the open side. */
	private final SurroundBeds<RainScene> beds;
	private RainScene scene = RainScene.DRY;
	/** A rate set by command, overriding the weather, or null. */
	private @Nullable Float forced;
	private int ticks;
	private int dryFor;

	public RainController(NativeBridge bridge) {
		this.bridge = bridge;
		this.beds = new SurroundBeds<>("rain", SoundSource.WEATHER, bridge, NativeBridge.BED_RAIN);
	}

	/** Whether Foley is making the rain, so the game's own rain sounds are skipped. */
	public boolean replacing() {
		return io.github.cumsauce.foley.FoleyClient.wants(io.github.cumsauce.foley.config.FoleyConfig::rain);
	}

	public RainScene scene() {
		return scene;
	}

	/** One of its beds, if it's playing. */
	public @Nullable BedSound<RainScene> sound() {
		return beds.any();
	}

	/** Forces the rain to {@code rate} mm/h whatever the weather, or back to the weather with null. */
	public void force(@Nullable Float rate) {
		forced = rate;
		ticks = 0;
	}

	public void tick(Minecraft minecraft) {
		if (minecraft.level == null) {
			stop(minecraft);
			return;
		}
		if (!replacing()) {
			stop(minecraft);
			return;
		}
		if (ticks++ % SAMPLE_EVERY != 0) return;
		RainScene sampled = RainScene.sample(minecraft);
		if (forced != null) {
			float[] surfaces = sampled.rate() > 0 ? sampled.surfaces() : RainScene.DRY.surfaces();
			sampled = new RainScene(forced, surfaces, sampled.roof(), sampled.height());
		}
		if (forced == null) {
			// Rain comes on over half a minute (first a scatter of drops, then the downpour) and eases off over
			// some ten seconds, rather than as fast as the sky darkens.
			float k = sampled.rate() > heardRate ? 0.025f : 0.06f;
			heardRate += (sampled.rate() - heardRate) * k;
			if (sampled.rate() <= 0 && heardRate < 0.05f) heardRate = 0;
			sampled = new RainScene(heardRate, sampled.surfaces(), sampled.roof(), sampled.height());
		}
		scene = sampled;
		if (scene.rate() <= 0) {
			dryFor += SAMPLE_EVERY;
			beds.setScene(scene);
			if (dryFor > LINGER) stop(minecraft);
			return;
		}
		dryFor = 0;
		WindController wind = io.github.cumsauce.foley.FoleyClient.wind();
		beds.keep(minecraft, scene, wind == null ? new float[SurroundBeds.AROUND] : wind.scene().around());
	}

	private void stop(Minecraft minecraft) {
		beds.stop(minecraft);
	}
}
