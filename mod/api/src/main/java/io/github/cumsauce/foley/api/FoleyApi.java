package io.github.cumsauce.foley.api;

import org.jspecify.annotations.Nullable;

/** Foley's synthesis, handed to each {@link FoleyEntrypoint}. Grows as more becomes shareable. */
public interface FoleyApi {
	/** Foley's sounds: the catalogue to render and play from. */
	FoleySounds sounds();

	/** Foley's materials: describe a block's physics and have Foley's contacts and breaks follow them. */
	FoleyMaterials materials();

	/** Foley's beds: a mod's own streaming sound, told its scene as its world changes. */
	FoleyBeds beds();

	/** Foley's creature voices: describe a modded creature's voice and Foley speaks for it. */
	FoleyVoices voices();

	/**
	 * Foley's vanilla sound overrides: take over any vanilla event Foley would have played, by event id. This is the
	 * general form of {@link #doorHandler(DoorSoundHandler)}; doors are simply one predicate away.
	 */
	SoundOverrideRegistry soundOverrides();

	/**
	 * Sets or clears the handler that takes over vanilla door and trapdoor sounds. While one is registered Foley does
	 * not play its built-in contact tap: each event is handed to the handler instead, and an unhandled one falls back to
	 * the game's sound. {@code null} restores the built-in tap. Client thread, at any time — not only during
	 * {@link FoleyEntrypoint}.
	 */
	void doorHandler(@Nullable DoorSoundHandler handler);

	/** The door sound handler in force, or null while Foley's built-in contact tap is. */
	@Nullable
	DoorSoundHandler doorHandler();

	/** Foley's own implementation of the API, found by ServiceLoader. Nothing else implements it. */
	interface Provider {
		/** Foley's API once its synthesis is ready, or null until then. */
		@Nullable FoleyApi api();
	}
}
