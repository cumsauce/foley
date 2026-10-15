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

	/** Foley's own implementation of the API, found by ServiceLoader. Nothing else implements it. */
	interface Provider {
		/** Foley's API once its synthesis is ready, or null until then. */
		@Nullable FoleyApi api();
	}
}
