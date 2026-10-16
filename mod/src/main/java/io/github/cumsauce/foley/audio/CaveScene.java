package io.github.cumsauce.foley.audio;

import java.util.Locale;

/** What the cave bed is told about the world: depth, mass, openness, drips, gain. */
public record CaveScene(float depth, float mass, float openness, float drips, float gain) implements Scene {
	public static final CaveScene SILENT = new CaveScene(0, 0, 1, 0, 0);

	@Override
	public float[] params() {
		return new float[] {depth, mass, openness, drips, gain};
	}

	@Override
	public String toString() {
		return String.format(Locale.ROOT, "CaveScene[depth=%.2f, mass=%.2f, openness=%.2f, drips=%.2f, gain=%.2f]",
				depth, mass, openness, drips, gain);
	}
}
