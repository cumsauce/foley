package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.api.FoleyApi;
import org.jspecify.annotations.Nullable;

/**
 * Foley's implementation of the API's {@link FoleyApi.Provider}, found by ServiceLoader. Only Foley itself calls
 * {@link #ready}; the {@code volatile} field keeps {@code Foley.available()} honest from any thread.
 */
public final class FoleyProvider implements FoleyApi.Provider {
	private static volatile @Nullable FoleyApi api;

	/** Public for ServiceLoader; only Foley constructs it. */
	public FoleyProvider() {}

	/** Foley's synthesis is up. Called by Foley itself, before its addons' entrypoints. */
	public static void ready(FoleyApi api) {
		FoleyProvider.api = api;
	}

	@Override
	public @Nullable FoleyApi api() {
		return api;
	}
}
