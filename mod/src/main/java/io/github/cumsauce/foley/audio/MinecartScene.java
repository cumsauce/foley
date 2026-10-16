package io.github.cumsauce.foley.audio;

/** A minecart's scene: how fast it's going, in m/s. */
public record MinecartScene(float speed) implements Scene {
	public float[] params() {
		return new float[] {speed};
	}
}
