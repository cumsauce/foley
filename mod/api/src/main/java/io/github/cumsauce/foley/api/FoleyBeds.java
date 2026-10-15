package io.github.cumsauce.foley.api;

import net.minecraft.resources.Identifier;

/**
 * A mod's own beds: Foley's long-running, streaming models — rain, wind, water, fire, lava, thunder — played where a
 * mod wants, told its scene as its own world or mechanics change. A bed is a stream, and the game allows only 8 at a
 * time; Foley's own take most, so at most {@link #MOST} beds addons may run at once, and {@link #start} refuses past
 * that.
 *
 * <p>All of it is client thread. Scenes glide over about a third of a second, so a scene changed every tick never
 * steps. What each kind's scene takes, floats in order:
 *
 * <ul>
 *   <li>{@link BedKind#RAIN}: the rate (mm/h, 0 to 200), the 7 surfaces' shares of what it lands on (water, stone,
 *       leaves, wood, soft, glass, metal), the roof's surface (−1 for none) and its height above the listener in
 *       blocks;</li>
 *   <li>{@link BedKind#WIND}: mean speed (m/s, 0 to 60), exposure (0 to 1), leaves (0 to 1);</li>
 *   <li>{@link BedKind#WATER}: blocks flowing, blocks where a waterfall lands, gain (0 to 1);</li>
 *   <li>{@link BedKind#FIRE} / {@link BedKind#LAVA}: blocks, gain;</li>
 *   <li>{@link BedKind#THUNDER}: air temperature (°C), humidity (percent), exposure (0 to 1).</li>
 * </ul>
 */
public interface FoleyBeds {
	/** How many beds addons may run at once. */
	int MOST = 2;

	/** The streaming models, each told a scene of floats. */
	enum BedKind {
		/** Rain: drops on what is round the listener, drumming on any roof. */
		RAIN,
		/** Wind: turbulence rising with speed and exposure. */
		WIND,
		/** Water: a stream's or waterfall's bubbles. */
		WATER,
		/** Fire: combustion roar and crackles. */
		FIRE,
		/** Lava: a rumble and bursting bubbles. */
		LAVA,
		/** Thunder: struck with lightning, its thunder arriving as sound would. */
		THUNDER
	}

	/**
	 * Starts bed {@code id} (in your mod's namespace) of {@code kind}, silent until its first scene, playing all
	 * round the listener until {@link #place}. Returns whether it started: false past {@link #MOST}, or if the id is
	 * already running, or a scene of the wrong length. Unlike the API's registrations, beds come and go whenever
	 * suits your mod — client thread.
	 */
	boolean start(Identifier id, BedKind kind, float[] scene);

	/** Tells the bed what the world is like now; glides there. Its length must fit the kind. */
	void scene(Identifier id, float[] scene);

	/** Plays it from {@code x}, {@code y}, {@code z} in the world from now on. */
	void place(Identifier id, double x, double y, double z);

	/** Heads for {@code weight} (0 to 1) times Foley's volume, over a second or so. */
	void weight(Identifier id, float weight);

	/** Whether {@code id} is running. */
	boolean playing(Identifier id);

	/** Stops {@code id} and frees its stream. Unknown ids are ignored. */
	void stop(Identifier id);
}
