package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.api.DoorSoundHandler;
import io.github.cumsauce.foley.api.FoleyApi;
import io.github.cumsauce.foley.api.FoleyBeds;
import io.github.cumsauce.foley.api.FoleyMaterials;
import io.github.cumsauce.foley.api.FoleyVoices;
import io.github.cumsauce.foley.api.FoleySounds;
import io.github.cumsauce.foley.api.SoundModel;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundSource;
import org.jspecify.annotations.Nullable;

/**
 * Foley's API for one addon: every registration it makes names it in any error, so a clash or a bad value reads as
 * whose it is. The facade ({@code Foley.api()}) gets the empty origin, which may only play: registration happens in
 * the entrypoint, where the addon's own instance is handed to it.
 */
final class FoleyApiImpl implements FoleyApi {
	private final String origin;

	FoleyApiImpl(String origin) {
		this.origin = origin;
	}

	@Override
	public FoleySounds sounds() {
		return new Addon(origin);
	}

	@Override
	public FoleyMaterials materials() {
		return CustomMaterials.api(origin);
	}

	@Override
	public FoleyBeds beds() {
		return AddonBeds.api(origin);
	}

	@Override
	public FoleyVoices voices() {
		return CustomVoices.api(origin);
	}

	@Override
	public void doorHandler(@Nullable DoorSoundHandler handler) {
		DoorHandling.handler(handler);
	}

	@Override
	public @Nullable DoorSoundHandler doorHandler() {
		return DoorHandling.handler();
	}

	private record Addon(String origin) implements FoleySounds {
		@Override
		public void register(Identifier id, SoundModel model, int variants) {
			if (origin.isEmpty())
				throw new IllegalStateException("sounds are registered from the foley entrypoint, not afterwards");
			AddonSounds.register(origin, id, model, variants);
		}

		@Override
		public boolean play(Identifier id, double x, double y, double z, SoundSource source, float volume,
				float pitch) {
			return AddonSounds.play(id, x, y, z, source, volume, pitch);
		}
	}
}
