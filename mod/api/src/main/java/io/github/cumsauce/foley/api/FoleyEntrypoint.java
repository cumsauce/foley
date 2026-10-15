package io.github.cumsauce.foley.api;

/**
 * A mod's integration with Foley. Declared in its {@code fabric.mod.json} under the {@code "foley"} entrypoint:
 *
 * <pre>{@code
 * "entrypoints": { "foley": [ "com.example.sounds.ExampleFoley" ] }
 * }</pre>
 *
 * <p>If Foley is absent, the loader ignores the entrypoint: integrations are soft dependencies by construction. It is
 * called once, on the client, when Foley's synthesis is ready; register sounds here, as registration closes when the
 * last addon returns. A throwing implementation is logged and skipped with its sounds; it never takes Foley or the
 * game down.
 */
public interface FoleyEntrypoint {
	/** Called once Foley's synthesis is ready. */
	void onFoleyReady(FoleyApi api);
}
