package io.github.cumsauce.foley.audio;

import java.util.Locale;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;

/**
 * The air thunder crosses and the listener's shelter: about 20 °C per unit of the biome's temperature (as Earshot
 * reckons it), near-saturated humidity in rain, and how open to the sky the listener is (from the wind's sampling).
 *
 * @param celsius  the air's temperature
 * @param humidity relative humidity, percent
 * @param exposure 0 shut in to 1 in the open
 */
public record ThunderScene(float celsius, float humidity, float exposure) implements Scene {
	public static final ThunderScene DEFAULT = new ThunderScene(15, 90, 1);

	public static ThunderScene sample(Minecraft minecraft, float exposure) {
		if (minecraft.level == null || minecraft.player == null) return DEFAULT;
		BlockPos at = minecraft.player.blockPosition();
		float celsius = Math.clamp(20 * minecraft.level.getBiome(at).value().getBaseTemperature(), -30, 50);
		float humidity = minecraft.level.isRaining() ? 95 : 60;
		return new ThunderScene(celsius, humidity, Math.clamp(exposure, 0, 1));
	}

	@Override
	public float[] params() {
		return new float[] {celsius, humidity, exposure};
	}

	@Override
	public String toString() {
		return String.format(Locale.ROOT, "Thunder[%.0f °C, %.0f %%, exposure=%.2f]", celsius, humidity, exposure);
	}
}
