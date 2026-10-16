package io.github.cumsauce.foley.audio;

/** The scene the underwater bed is told: a single {@code level} 0..=1. */
public record UnderwaterScene(float level) implements Scene {
	public static final UnderwaterScene DRY = new UnderwaterScene(0f);

	@Override
	public float[] params() {
		return new float[] {level};
	}

	@Override
	public String toString() {
		return String.format(java.util.Locale.ROOT, "UnderwaterScene[level=%.2f]", level);
	}
}