package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.api.DoorSoundHandler;
import io.github.cumsauce.foley.api.SoundOverrideRegistration;
import io.github.cumsauce.foley.api.SoundOverrideRegistry;
import io.github.cumsauce.foley.api.VanillaSoundOverride;
import java.util.List;
import java.util.Objects;
import java.util.Set;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.function.Predicate;
import net.minecraft.resources.Identifier;
import org.jspecify.annotations.Nullable;

/**
 * S1/S2: which sound a vanilla event gets. Foley's own contact tap by default for doors; an addon's
 * {@link DoorSoundHandler}, once one is registered, takes door events instead of the tap and may decline them (then
 * the game's sound plays). More generally, addons may register {@link VanillaSoundOverride}s for any event that
 * {@link Replacements} consults.
 *
 * <p>{@link #dispatch} is the general seam: overrides are consulted last-registered first, and the first whose
 * predicate matches decides the event. {@link #action} is the legacy door-only decision, kept for the built-in tap's
 * tests; {@link Replacements} now calls {@link #dispatch} and falls back to its own logic on {@link Action#NONE}.
 *
 * <p>The handler field and the override list are safe to set from any client-thread moment, not only an entrypoint:
 * the field is {@code volatile} and the list is copy-on-write.
 */
final class DoorHandling {
	/** What {@link Replacements} does with an event. */
	enum Action {
		/** An override took the event; the game's sound stays skipped and no built-in sound plays. */
		OVERRIDE_HANDLED,
		/** An override declined the event; let the game's sound play. */
		OVERRIDE_VANILLA,
		/** No override matched; Foley's own behaviour applies. */
		NONE,
		/** @deprecated legacy vocabulary of {@link #action}; use {@link #NONE} and friends. */
		@Deprecated
		PLAY_VANILLA,
		/** @deprecated legacy vocabulary of {@link #action}; use {@link #NONE} and friends. */
		@Deprecated
		CONTACT_TAP,
		/** @deprecated legacy vocabulary of {@link #action}; use {@link #OVERRIDE_HANDLED} and friends. */
		@Deprecated
		ADDON
	}

	/** The game's door and trapdoor events, wooden and iron: the ones Foley's contact tap stands in for. */
	private static final Set<Identifier> DOORS = Set.of(Identifier.fromNamespaceAndPath("minecraft", "block.wooden_door.open"),
			Identifier.fromNamespaceAndPath("minecraft", "block.wooden_door.close"),
			Identifier.fromNamespaceAndPath("minecraft", "block.wooden_trapdoor.open"),
			Identifier.fromNamespaceAndPath("minecraft", "block.wooden_trapdoor.close"),
			Identifier.fromNamespaceAndPath("minecraft", "block.iron_door.open"),
			Identifier.fromNamespaceAndPath("minecraft", "block.iron_door.close"),
			Identifier.fromNamespaceAndPath("minecraft", "block.iron_trapdoor.open"),
			Identifier.fromNamespaceAndPath("minecraft", "block.iron_trapdoor.close"));

	/** The game's iron door and trapdoor events: exactly the iron members of {@link #DOORS}. */
	private static final Set<Identifier> IRON_DOORS = Set.of(
			Identifier.fromNamespaceAndPath("minecraft", "block.iron_door.open"),
			Identifier.fromNamespaceAndPath("minecraft", "block.iron_door.close"),
			Identifier.fromNamespaceAndPath("minecraft", "block.iron_trapdoor.open"),
			Identifier.fromNamespaceAndPath("minecraft", "block.iron_trapdoor.close"));

	/** Registered overrides, in registration order; dispatch walks them backwards. */
	private static final List<Entry> OVERRIDES = new CopyOnWriteArrayList<>();

	private static volatile @Nullable DoorSoundHandler handler;
	private static volatile @Nullable SoundOverrideRegistration handlerRegistration;

	private DoorHandling() {}

	/** Whether {@code event} is one of the game's door or trapdoor sounds (wooden or iron, open or close). */
	static boolean isDoorEvent(Identifier event) {
		return DOORS.contains(event);
	}

	/** Whether {@code event} is the iron variant of a door or trapdoor sound. */
	static boolean isIronDoorEvent(Identifier event) {
		return IRON_DOORS.contains(event);
	}

	/**
	 * Runs the overrides for one event.
	 *
	 * @return {@link Action#OVERRIDE_HANDLED} if an override took it, {@link Action#OVERRIDE_VANILLA} if an override
	 *         declined it, or {@link Action#NONE} if none matched
	 */
	static Action dispatch(Identifier event, double x, double y, double z) {
		// Last registered wins: an addon that registers later overrides an earlier one, and the door sugar is just
		// another registration.
		for (int i = OVERRIDES.size() - 1; i >= 0; i--) {
			Entry override = OVERRIDES.get(i);
			if (!override.match.test(event)) continue;
			return override.handler.onVanillaSound(event, x, y, z) ? Action.OVERRIDE_HANDLED
					: Action.OVERRIDE_VANILLA;
		}
		return Action.NONE;
	}

	/** Registers an override; the returned handle removes it again. */
	static SoundOverrideRegistration register(Predicate<Identifier> match, VanillaSoundOverride handler) {
		Objects.requireNonNull(match, "match");
		Objects.requireNonNull(handler, "handler");
		Entry override = new Entry(match, handler);
		OVERRIDES.add(override);
		return new Registration(override);
	}

	/** Removes every override, including the door handler's sugar registration. */
	static synchronized void clearOverrides() {
		OVERRIDES.clear();
		handlerRegistration = null;
		handler = null;
	}

	/** The registry the API hands out; register and clear are {@link #register} and {@link #clearOverrides}. */
	static SoundOverrideRegistry soundOverrides() {
		return REGISTRY;
	}

	private static final SoundOverrideRegistry REGISTRY = new SoundOverrideRegistry() {
		@Override
		public SoundOverrideRegistration register(Predicate<Identifier> match, VanillaSoundOverride handler) {
			return DoorHandling.register(match, handler);
		}

		@Override
		public void clear() {
			DoorHandling.clearOverrides();
		}
	};

	/** The addon handler in force, or null while Foley's built-in tap is. */
	static @Nullable DoorSoundHandler handler() {
		return handler;
	}

	/**
	 * Sets the handler; null restores Foley's built-in tap. The handler is sugar over {@link #register}: it matches the
	 * game's door events, derives the iron flag from the event id, and delegates.
	 */
	static synchronized void handler(@Nullable DoorSoundHandler handler) {
		SoundOverrideRegistration prior = handlerRegistration;
		handlerRegistration = null;
		if (prior != null) prior.unregister();
		DoorHandling.handler = handler;
		if (handler != null) {
			handlerRegistration = register(DoorHandling::isDoorEvent,
					(event, x, y, z) -> handler.onVanillaDoorEvent(event, x, y, z, isIronDoorEvent(event)));
		}
	}

	/**
	 * The legacy door-only action.
	 *
	 * @param bypassed      Foley's A/B bypass: wins over everything, and the handler is not consulted
	 * @param contactWanted whether Foley can play its tap at all (its structures system is up and the doors setting on)
	 * @param hasHandler    whether a handler is registered
	 * @param handled       what the handler returned; only read when {@code hasHandler}
	 */
	@Deprecated
	static Action action(boolean bypassed, boolean contactWanted, boolean hasHandler, boolean handled) {
		if (bypassed) return Action.PLAY_VANILLA;
		if (hasHandler) return handled ? Action.ADDON : Action.PLAY_VANILLA;
		return contactWanted ? Action.CONTACT_TAP : Action.PLAY_VANILLA;
	}

	/** One registration. Identity equality, so a handle removes exactly its own entry. */
	private static final class Entry {
		final Predicate<Identifier> match;
		final VanillaSoundOverride handler;

		Entry(Predicate<Identifier> match, VanillaSoundOverride handler) {
			this.match = match;
			this.handler = handler;
		}
	}

	private static final class Registration implements SoundOverrideRegistration {
		private final Entry override;
		private boolean registered = true;

		Registration(Entry override) {
			this.override = override;
		}

		@Override
		public synchronized void unregister() {
			if (!registered) return;
			registered = false;
			OVERRIDES.remove(override);
		}
	}
}
