package io.github.cumsauce.foley.api;

/**
 * A sound Foley makes, as a value: one of its synthesis models with the parameters this one takes. The same value
 * stands behind registration ({@link FoleySounds#register}) and the JSON form ({@code assets/<mod>/foley/sounds.json},
 * through {@link SoundCodecs}), so a sound can be defined in code or in a resource pack.
 *
 * <p>Each record checks its parameters against what the native synthesis accepts ({@code foley.h}'s ranges), so a bad
 * value fails where it is built, not when it is played. New models are added over time; existing ones keep their
 * fields.
 */
public sealed interface SoundModel {
	/** A drop falling into water, its bubble ringing as Minnaert found ({@code radius_mm} 0.1 to 10). */
	record DripIntoWater(float radiusMm) implements SoundModel {
		public DripIntoWater {
			radiusMm = SoundModel.check("radius_mm", radiusMm, 0.1f, 10f);
		}
	}

	/** A drop landing on stone ({@code radius_mm} 0.1 to 10): a tick through stone's modes, with a bright splash. */
	record DripOnStone(float radiusMm) implements SoundModel {
		public DripOnStone {
			radiusMm = SoundModel.check("radius_mm", radiusMm, 0.1f, 10f);
		}
	}

	/** A drop into lava: a thud and a sizzle. */
	record LavaDrip() implements SoundModel {}

	/** Crickets chirping ({@code celsius} −30 to 50), at Dolbear's rate for the temperature. */
	record Crickets(float celsius) implements SoundModel {
		public Crickets {
			celsius = SoundModel.check("celsius", celsius, -30f, 50f);
		}
	}

	/** A bird's song ({@code species} 0 to 1000): a syrinx model, each species its own syllables. */
	record Birdsong(int species) implements SoundModel {
		public Birdsong {
			SoundModel.check("species", species, 0, 1000);
		}
	}

	/** A frog calling ({@code length_cm} 2 to 20): deeper the bigger the frog. */
	record Frog(float lengthCm) implements SoundModel {
		public Frog {
			lengthCm = SoundModel.check("length_cm", lengthCm, 2f, 20f);
		}
	}

	/** A wave breaking ({@code wave_height_m} 0.2 to 3). */
	record Surf(float waveHeightM) implements SoundModel {
		public Surf {
			waveHeightM = SoundModel.check("wave_height_m", waveHeightM, 0.2f, 3f);
		}
	}

	/** A woodpecker drumming on a tree. */
	record Woodpecker() implements SoundModel {}

	/** An owl hooting. */
	record Owl() implements SoundModel {}

	/** Cicadas singing ({@code celsius} −30 to 50), as the temperature sets their rate. */
	record Cicadas(float celsius) implements SoundModel {
		public Cicadas {
			celsius = SoundModel.check("celsius", celsius, -30f, 50f);
		}
	}

	/** A wolf howling. */
	record Howl() implements SoundModel {}

	/** Brushing through plants: grass, leaves or dry stems. */
	record Brush(PlantKind plant) implements SoundModel {}

	/** Lake ice singing: a skate's glide setting the ice's modes ringing. */
	record IceSinging() implements SoundModel {}

	/** Stones settling in a cave: a small rockfall's clicks. */
	record SettlingStone() implements SoundModel {}

	/** A bird flushed off the ground: wings and a startled call. */
	record Flush() implements SoundModel {}

	/** An insect's wingbeat buzz: a bee's or a fly's. */
	record Buzz(InsectKind insect) implements SoundModel {}

	/** A sculk sensor's triggered tone ({@code q} 0.5 to 20): a bowl ring at ~200 Hz, the higher the q the longer. */
	record SculkRing(float q) implements SoundModel {
		public SculkRing {
			q = check("q", q, 0.5f, 20f);
		}
	}

	/** A bubble's Minnaert ring ({@code radius_mm} 0.05 to 20), gone in tens of milliseconds. */
	record Bubble(float radiusMm) implements SoundModel {
		public Bubble {
			radiusMm = check("radius_mm", radiusMm, 0.05f, 20f);
		}
	}

	/**
	 * A burst of noise through a band {@code q} wide round {@code centre_hz}, over {@code seconds}, peaking at
	 * {@code gain}: the texture of splashes, hisses, crunches and whooshes.
	 */
	record NoiseBurst(float centreHz, float q, float seconds, float gain) implements SoundModel {
		public NoiseBurst {
			check("centre_hz", centreHz, 20f, 20_000f);
			check("q", q, 0.5f, 20f);
			check("seconds", seconds, 0.005f, 5f);
			check("gain", gain, 0f, 1f);
		}
	}

	/** A tone at {@code centre_hz} gliding to {@code target_hz} over {@code seconds}, peaking at {@code gain}. */
	record Tone(float centreHz, float targetHz, float seconds, float gain) implements SoundModel {
		public Tone {
			check("centre_hz", centreHz, 20f, 10_000f);
			check("target_hz", targetHz, 20f, 20_000f);
			check("seconds", seconds, 0.01f, 5f);
			check("gain", gain, 0f, 1f);
		}
	}

	/** A gust at {@code speed} m/s over {@code seconds}, peaking at {@code gain} (the wind's own loudness applies). */
	record Gust(float speed, float seconds, float gain) implements SoundModel {
		public Gust {
			check("speed", speed, 0f, 60f);
			check("seconds", seconds, 0.2f, 10f);
			check("gain", gain, 0f, 1f);
		}
	}

	/** A body meeting a material ({@code speed} 0.1 to 30 m/s): a tap, a block set down, a landing, a footstep. */
	record Contact(ContactKind kind, MaterialKind material, float speed) implements SoundModel {
		public Contact {
			speed = SoundModel.check("speed", speed, 0.1f, 30f);
		}
	}

	/**
	 * A mob's call, in Foley's voice for it: {@code mob} and {@code call} are its names ({@code "cow"},
	 * {@code "ambient"}), validated against Foley's tables when registered. A call a mob doesn't have is its ambient
	 * one.
	 */
	record Voice(String mob, String call) implements SoundModel {
		public Voice {
			if (mob == null || !mob.matches("[a-z0-9_]+")) throw new IllegalArgumentException("mob must be a name like \"cow\"");
			if (call == null || !call.matches("[a-z0-9_]+")) throw new IllegalArgumentException("call must be a name like \"ambient\"");
		}
	}

	/** What is brushed through. */
	enum PlantKind {
		/** Grass. */
		GRASS,
		/** Leafy branches. */
		LEAVES,
		/** Dry stems. */
		DRY
	}

	/** What buzzes. */
	enum InsectKind {
		/** A bee, about 230 Hz. */
		BEE,
		/** A fly, faster and raspier. */
		FLY
	}

	/** What a body meets: Foley's ten materials. */
	enum MaterialKind {
		/** Rock of any kind. */
		STONE,
		/** Planks, logs, anything wooden. */
		WOOD,
		/** Iron, copper, anvils. */
		METAL,
		/** Glass and amethyst. */
		GLASS,
		/** Ice, packed and thin. */
		ICE,
		/** Dirt, grass, clay, mud. */
		EARTH,
		/** Sand and gravel. */
		GRANULAR,
		/** Leaves, plants, crops. */
		PLANT,
		/** Wool, slime, carpets. */
		SOFT,
		/** Settled snow, which crunches and squeaks. */
		SNOW
	}

	/** How a body meets a material. */
	enum ContactKind {
		/** A swung tool tapping. */
		TAP,
		/** A block set down. */
		PLACE,
		/** A body landing. */
		LAND,
		/** The block giving way; no speed. */
		BREAK,
		/** A heel strike. */
		STEP
	}

	private static float check(String name, float value, float min, float max) {
		if (!(min <= value && value <= max))
			throw new IllegalArgumentException(name + " must be " + min + " to " + max + ", not " + value);
		return value;
	}

	private static void check(String name, int value, int min, int max) {
		if (!(min <= value && value <= max))
			throw new IllegalArgumentException(name + " must be " + min + " to " + max + ", not " + value);
	}
}
