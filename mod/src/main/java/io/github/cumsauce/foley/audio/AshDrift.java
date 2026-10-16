package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import net.minecraft.client.Minecraft;
import net.minecraft.tags.BiomeTags;
import net.minecraft.util.RandomSource;

/**
 * High-frequency bandpassed noise during a blizzard or ash storm (M22). Almost inaudible but the right texture: a
 * wash of 6 kHz bandpassed noise that ticks every 4 to 8 s. Reuses {@link NativeBridge#PRIMITIVE_NOISE} for the
 * sound and {@link BufferSound} to play it. Stays silent indoors (in any dimension with one) and under trees.
 */
public final class AshDrift {
	/** Centre 6 kHz, q 4, seconds 0.4, gain 0.04 — the spec's texture. */
	public static final float[] PARAMS = {6000.0f, 4.0f, 0.4f, 0.04f};
	/** The shortest gap between two drifts (ticks). */
	public static final int MIN_INTERVAL_TICKS = 80;
	/** The longest (ticks). */
	public static final int MAX_INTERVAL_TICKS = 200;

	private final RandomSource random = RandomSource.create();
	private int ticks;
	private int next;

	public void tick(Minecraft minecraft) {
		ticks++;
		if (minecraft.level == null || minecraft.player == null) return;
		if (!FoleyClient.wants(FoleyConfig::ashDrift)) return;
		if (minecraft.level.dimensionType().hasCeiling()) return;
		var biome = minecraft.level.getBiome(minecraft.player.blockPosition());
		boolean blizzard = minecraft.level.isRaining() && biome.is(BiomeTags.IS_TAIGA);
		// "Ash storm" stand-in: a thunderstorm over hot biomes — the badlands feel.
		boolean ashStorm = minecraft.level.isThundering() && (biome.is(BiomeTags.IS_BADLANDS)
				|| biome.is(BiomeTags.IS_SAVANNA));
		if (!blizzard && !ashStorm) return;
		if (ticks < next) return;
		next = ticks + MIN_INTERVAL_TICKS + random.nextInt(MAX_INTERVAL_TICKS - MIN_INTERVAL_TICKS);
		var bridge = FoleyClient.bridge();
		if (bridge == null) return;
		short[] pcm = bridge.renderPrimitive(io.github.cumsauce.foley.natives.NativeBridge.PRIMITIVE_NOISE, PARAMS, 0L);
		if (pcm == null || pcm.length == 0) return;
		var ear = minecraft.player.getEyePosition();
		float volume = 0.4f * FoleyClient.gain();
		minecraft.getSoundManager().play(new BufferSound(Sounds.event("ash_drift").location(), volume, ear.x, ear.y,
				ear.z, pcm));
	}
}