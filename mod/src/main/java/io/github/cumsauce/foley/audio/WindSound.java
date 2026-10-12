package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.natives.NativeBridge;
import java.lang.foreign.MemorySegment;
import net.minecraft.sounds.SoundSource;

/** The wind round the listener, synthesized as it plays (M1), following the weather, height and shelter. */
public final class WindSound extends BedSound<WindScene> {
	public WindSound(NativeBridge bridge, WindScene scene) {
		super("wind", SoundSource.AMBIENT, bridge, scene);
	}

	@Override
	MemorySegment create(long seed) {
		return bridge.windCreate(seed);
	}

	@Override
	void apply(MemorySegment wind, WindScene scene) {
		bridge.windSetScene(wind, scene.speed(), scene.exposure(), scene.leaves());
	}

	@Override
	void render(MemorySegment wind, MemorySegment out) {
		bridge.windRender(wind, out);
	}

	@Override
	void destroy(MemorySegment wind) {
		bridge.windDestroy(wind);
	}
}
