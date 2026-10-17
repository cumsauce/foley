package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.api.DoorSoundHandler;
import org.jspecify.annotations.Nullable;

/**
 * S1: which sound a vanilla door or trapdoor event gets. Foley's own contact tap by default; an addon's
 * {@link DoorSoundHandler}, once one is registered, takes the event instead of the tap and may decline it (then the
 * game's sound plays).
 *
 * <p>The decision is pure so it can be tested without the game's registries; {@link Replacements} does the playing.
 * The handler field is {@code volatile}: the API may set it from any client-thread moment, not only an entrypoint.
 */
final class DoorHandling {
	/** What {@link Replacements} does with a door event. */
	enum Action {
		/** Let the game's sound play. */
		PLAY_VANILLA,
		/** Skip it and play Foley's built-in contact tap. */
		CONTACT_TAP,
		/** Skip it; the registered handler plays whatever it likes. */
		ADDON
	}

	private static volatile @Nullable DoorSoundHandler handler;

	private DoorHandling() {}

	/** The addon handler in force, or null while Foley's built-in tap is. */
	static @Nullable DoorSoundHandler handler() {
		return handler;
	}

	/** Sets the handler; null restores Foley's built-in tap. */
	static void handler(@Nullable DoorSoundHandler handler) {
		DoorHandling.handler = handler;
	}

	/**
	 * The action for one door event.
	 *
	 * @param bypassed      Foley's A/B bypass: wins over everything, and the handler is not consulted
	 * @param contactWanted whether Foley can play its tap at all (its structures system is up and the doors setting on)
	 * @param hasHandler    whether a handler is registered
	 * @param handled       what the handler returned; only read when {@code hasHandler}
	 */
	static Action action(boolean bypassed, boolean contactWanted, boolean hasHandler, boolean handled) {
		if (bypassed) return Action.PLAY_VANILLA;
		if (hasHandler) return handled ? Action.ADDON : Action.PLAY_VANILLA;
		return contactWanted ? Action.CONTACT_TAP : Action.PLAY_VANILLA;
	}
}
