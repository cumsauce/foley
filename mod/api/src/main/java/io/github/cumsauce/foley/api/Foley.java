package io.github.cumsauce.foley.api;

import io.github.cumsauce.foley.api.FoleyApi.Provider;
import java.util.ServiceLoader;
import net.fabricmc.loader.api.FabricLoader;
import org.jspecify.annotations.Nullable;

/**
 * Another mod's way into Foley's synthesis, on the client. All of it is client-side: servers never load it, and a
 * client without Foley (or with its synthesis down) simply reports {@link #available()} false.
 */
public final class Foley {
	/** Foley's implementation, found by ServiceLoader; absent when Foley isn't installed. */
	private static final @Nullable Provider provider = ServiceLoader.load(Provider.class)
			.findFirst()
			.orElse(null);

	private Foley() {}

	/** Whether Foley's synthesis is running: its natives loaded and its addons' entrypoints done. */
	public static boolean available() {
		return apiOrNull() != null;
	}

	/** The version of this API, as the loader reports the {@code foley-api} mod ({@code "dev"} in a workspace). */
	public static String version() {
		return FabricLoader.getInstance()
				.getModContainer("foley-api")
				.map(container -> container.getMetadata().getVersion().getFriendlyString())
				.orElse("dev");
	}

	/**
	 * Foley's API. Throws {@link IllegalStateException} if the synthesis is down — check {@link #available()} first,
	 * or keep the {@link FoleyEntrypoint} this came from.
	 */
	public static FoleyApi api() {
		FoleyApi api = apiOrNull();
		if (api == null)
			throw new IllegalStateException("Foley's synthesis is not running (not installed, or its natives failed)");
		return api;
	}

	private static @Nullable FoleyApi apiOrNull() {
		return provider == null ? null : provider.api();
	}
}
