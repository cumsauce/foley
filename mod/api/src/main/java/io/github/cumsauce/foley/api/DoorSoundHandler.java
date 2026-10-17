package io.github.cumsauce.foley.api;

import net.minecraft.resources.Identifier;

/**
 * Takes over the game's door and trapdoor sounds from Foley. Register it with
 * {@link FoleyApi#doorHandler(DoorSoundHandler)} and Foley stops playing its built-in contact tap: every vanilla door
 * or trapdoor open/close event Foley would have skipped is handed to this instead.
 *
 * <p>The handler runs on the client thread, after Foley's bypass check. While Foley is bypassed the game's sound plays
 * and the handler is not called. Open and close are told apart by {@code vanillaEvent}'s path.
 */
@FunctionalInterface
public interface DoorSoundHandler {
	/**
	 * A vanilla door or trapdoor open or close event was intercepted and will not play. Return true if you handled it
	 * (the event stays skipped), or false to let the vanilla sound play as a fallback.
	 *
	 * @param vanillaEvent the game's event id, e.g. {@code minecraft:block.wooden_door.open} or
	 *                     {@code minecraft:block.iron_trapdoor.close}
	 * @param x            the event's world position
	 * @param y            the event's world position
	 * @param z            the event's world position
	 * @param iron         whether the door or trapdoor was iron rather than wood
	 */
	boolean onVanillaDoorEvent(Identifier vanillaEvent, double x, double y, double z, boolean iron);
}
