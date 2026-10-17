package io.github.cumsauce.foley.api;

import java.util.function.Predicate;
import net.minecraft.resources.Identifier;

/**
 * Foley's vanilla sound overrides: decide, per vanilla event id, whether Foley hands the event to your handler instead
 * of playing its own sound. Reach it with {@link FoleyApi#soundOverrides()}.
 *
 * <p>Registrations are considered in the order they were made; the first one whose predicate matches is the one asked,
 * and an unhandled event falls back to the game's sound. Handlers run on the client thread, at any time — not only
 * during {@link FoleyEntrypoint}.
 */
public interface SoundOverrideRegistry {
	/**
	 * Registers an override for every vanilla event id the predicate matches.
	 *
	 * @param match   which vanilla events this handler sees
	 * @param handler what to do with them
	 * @return a registration that removes the override when {@link SoundOverrideRegistration#unregister()} is called
	 */
	SoundOverrideRegistration register(Predicate<Identifier> match, VanillaSoundOverride handler);

	/** Removes every override, restoring Foley's own sounds. */
	void clear();
}
