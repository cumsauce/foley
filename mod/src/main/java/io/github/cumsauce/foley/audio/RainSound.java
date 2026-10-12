package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.natives.NativeBridge;
import java.lang.foreign.MemorySegment;
import net.minecraft.sounds.SoundSource;

/** The rain round the listener, synthesized as it plays (M1), following the weather and what the rain lands on. */
public final class RainSound extends BedSound<RainScene> {
	public RainSound(NativeBridge bridge, RainScene scene) {
		super("rain", SoundSource.WEATHER, bridge, scene);
	}

	@Override
	MemorySegment create(long seed) {
		return bridge.rainCreate(seed);
	}

	@Override
	void apply(MemorySegment rain, RainScene scene) {
		bridge.rainSetScene(rain, scene.rate(), scene.surfaces(), scene.roof(), scene.height());
	}

	@Override
	void render(MemorySegment rain, MemorySegment out) {
		bridge.rainRender(rain, out);
	}

	@Override
	void destroy(MemorySegment rain) {
		bridge.rainDestroy(rain);
	}
}
