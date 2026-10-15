package io.github.cumsauce.foley.api;

import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundSource;

/**
 * Foley's catalogue of sounds, as another mod uses it: define a sound once (in code here, or in
 * {@code assets/<mod>/foley/sounds.json} for a resource pack), then play it in the world.
 *
 * <p>A registered sound needs no sounds.json of its own and no sound files: its variants are synthesized the first
 * time each is asked for, on the game's background executor, and cached as ordinary sound buffers. It plays through
 * the game's own pool of static sources, from where it is, so it pans and Earshot shapes it as any sound. It follows
 * Foley's volume and its A/B bypass, and shares Foley's per-tick share of one-shots so addons cannot starve the
 * game's channels.
 */
public interface FoleySounds {
	/** How many variants a sound has when it doesn't say: the same as Foley's own. */
	int DEFAULT_VARIANTS = 8;

	/**
	 * Defines {@code id} (in your mod's namespace: {@code yourmod:a_name}) as {@code model}, with
	 * {@link #DEFAULT_VARIANTS} variants, each its own rendering. Only during {@link FoleyEntrypoint}; throws
	 * {@link IllegalStateException} after.
	 */
	default void register(Identifier id, SoundModel model) {
		register(id, model, DEFAULT_VARIANTS);
	}

	/**
	 * Defines {@code id} as {@code model} with {@code variants} renderings (1 to 64) the game picks among at random.
	 * The id's namespace must be your mod's. Only during {@link FoleyEntrypoint}; throws
	 * {@link IllegalStateException} after.
	 */
	void register(Identifier id, SoundModel model, int variants);

	/**
	 * Plays {@code id} at ({@code x}, {@code y}, {@code z}) at {@code volume} (1 as Foley's own sounds) and
	 * {@code pitch}, from the game's static sources. Returns whether it played: false while bypassed, at zero volume,
	 * or past Foley's share of one-shots this tick. Client thread; {@code id} must be defined.
	 *
	 * @throws IllegalArgumentException if {@code id} was never defined
	 */
	boolean play(Identifier id, double x, double y, double z, SoundSource source, float volume, float pitch);
}
