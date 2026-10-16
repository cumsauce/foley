package io.github.cumsauce.foley.audio;

import net.minecraft.world.phys.Vec3;

/** A boat's scene: where to lap, and how big the bubble is. */
public record BoatScene(float radiusMm, Vec3 at) implements Scene {
	public float[] params() {
		return new float[] {radiusMm};
	}
}
