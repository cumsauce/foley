package io.github.cumsauce.foley.audio;

/**
 * What the biome bed is told about the listener's biome (M19). One streaming
 * bed, one of seven kinds, with a per-kind {@code sub} count (vortex rings
 * per 30 s for desert, scuttle count for deep_dark, mangrove leaf count for
 * mangrove, leaf exposure for snow; unused for the others).
 *
 * @param kind      which biome's bed to render (0..=6, see the constants)
 * @param intensity 0 to 1: how much of the biome the listener is in
 * @param sub       kind-specific count (0 to 200)
 * @param gain      0 to 1: the bed's master gain
 */
public record BiomeScene(int kind, float intensity, float sub, float gain) implements Scene {
	/** Desert, with its granular sand hiss and dust devils. */
	public static final int DESERT = 0;
	/** Mushroom fields, with their low ambient hum. */
	public static final int MUSHROOM = 1;
	/** Lush caves, with their moss hiss. */
	public static final int LUSH = 2;
	/** Dripstone caves, with their drips. */
	public static final int DRIPSTONE = 3;
	/** Deep dark, with its subsonic hum and sculk scuttles. */
	public static final int DEEP_DARK = 4;
	/** Snow / ice spikes / frozen ocean, with the high-frequency hush. */
	public static final int SNOW = 5;
	/** Mangrove swamp, with its humid insect drone. */
	public static final int MANGROVE = 6;

	/** An off scene: the bed is silent. */
	public static final BiomeScene SILENT = new BiomeScene(DESERT, 0f, 0f, 0f);

	@Override
	public float[] params() {
		return new float[] {kind, intensity, sub, gain};
	}
}
