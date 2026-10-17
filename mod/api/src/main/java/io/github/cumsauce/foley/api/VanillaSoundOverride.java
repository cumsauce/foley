package io.github.cumsauce.foley.api;

import net.minecraft.resources.Identifier;

/**
 * Takes over a vanilla sound event from Foley. Register one with
 * {@link SoundOverrideRegistry#register(java.util.function.Predicate, VanillaSoundOverride)}; every event Foley would
 * have played and that the predicate matches is handed to this instead.
 *
 * <p>The handler runs on the client thread, after Foley's bypass check. While Foley is bypassed the game's sound plays
 * and the handler is not called.
 */
@FunctionalInterface
public interface VanillaSoundOverride {
	/**
	 * A vanilla sound event was intercepted and will not play. Return true if you handled it (the event stays
	 * skipped), or false to let the vanilla sound play as a fallback.
	 *
	 * @param vanillaEvent the game's event id, e.g. {@code minecraft:block.wooden_door.open}
	 * @param x            the event's world position
	 * @param y            the event's world position
	 * @param z            the event's world position
	 */
	boolean onVanillaSound(Identifier vanillaEvent, double x, double y, double z);
}
