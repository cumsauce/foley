package io.github.cumsauce.foley.audio;

/**
 * What the surf bed is told about the shore near the listener (M17): one streaming bed at the
 * ear, a continuous wash under grains of breaking wave. The native bed takes these four floats
 * in this order.
 *
 * @param height    wave height, metres, 0..=3 (0 renders silence): a lake's lapping, a calm sea,
 *                  a rough one
 * @param shore     1 at the water's edge, falling to 0 at the scan radius
 * @param storm     0 clear to 1 full thunderstorm
 * @param exposure  0 to 1: the wind bed's reported speed scaled by {@link SurfController#exposureFor}
 */
public record SurfScene(float height, float shore, float storm, float exposure) implements Scene {
	/** An off scene: the bed is silent. */
	public static final SurfScene SILENT = new SurfScene(0f, 0f, 0f, 0f);

	@Override
	public float[] params() {
		return new float[] {height, shore, storm, exposure};
	}
}
