package io.github.cumsauce.foley.audio;

import com.mojang.blaze3d.audio.SoundBuffer;
import io.github.cumsauce.foley.natives.NativeBridge;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import net.minecraft.resources.Identifier;
import org.jspecify.annotations.Nullable;

/**
 * One-shots rendered into ordinary sound buffers, under names in {@code assets/foley/sounds.json} that have no file
 * ({@code foley:generated/<model>_<n>}), so they play from the game's large pool of static sources like any sound. The
 * game asks its buffer library for them; {@code SoundBufferLibraryMixin} answers with {@link #render}. Each name is a
 * different variation (a seed), and the game picks among them at random.
 */
public final class GeneratedSounds {
	public static final String PREFIX = "generated/";
	private static @Nullable NativeBridge bridge;

	private GeneratedSounds() {}

	public static void use(NativeBridge bridge) {
		GeneratedSounds.bridge = bridge;
	}

	/** Whether {@code sound} (a sound's location, as sounds.json names it) is one Foley generates. */
	public static boolean isGenerated(Identifier sound) {
		return sound.getNamespace().equals("foley") && sound.getPath().startsWith(PREFIX);
	}

	/** A bird's song, a cricket's chirping, a frog's call, a wave, … ({@link Life}), or {@code otherwise}. */
	private static short @org.jspecify.annotations.Nullable [] life(NativeBridge bridge, String model, int variant,
			short @org.jspecify.annotations.Nullable [] otherwise) {
		if (model.startsWith("voice_")) {
			// voice_<mob>_<call>, where both may have underscores: the longest mob that fits.
			String rest = model.substring("voice_".length());
			String mob = Voices.MOBS.stream().filter(m -> rest.startsWith(m + "_"))
					.max(java.util.Comparator.comparingInt(String::length)).orElse(null);
			if (mob == null) return otherwise;
			int call = Voices.CALLS.indexOf(rest.substring(mob.length() + 1));
			if (call < 0) return otherwise;
			int m = Voices.MOBS.indexOf(mob);
			return bridge.renderVoice(m, call, 0x701CEL * (m + 1) + 31L * call + variant);
		}
		int split = model.lastIndexOf('_');
		if (split < 0) return otherwise;
		int n;
		try {
			n = Integer.parseInt(model.substring(split + 1));
		} catch (NumberFormatException e) {
			return otherwise;
		}
		long seed = 0x11FEL * (n + 1) + variant;
		return switch (model.substring(0, split)) {
			case "bird" -> bridge.renderOneShot(NativeBridge.SHOT_BIRDSONG, n, seed);
			case "crickets" -> bridge.renderOneShot(NativeBridge.SHOT_CRICKETS, Life.CRICKET_CELSIUS[n], seed);
			case "frog" -> bridge.renderOneShot(NativeBridge.SHOT_FROG, Life.FROG_CM[n], seed);
			case "surf" -> bridge.renderOneShot(NativeBridge.SHOT_SURF, Life.SURF_M[n], seed);
			case "woodpecker" -> bridge.renderOneShot(NativeBridge.SHOT_WOODPECKER, 0, seed);
			case "owl" -> bridge.renderOneShot(NativeBridge.SHOT_OWL, 0, seed);
			case "cicadas" -> bridge.renderOneShot(NativeBridge.SHOT_CICADAS, Life.CICADA_CELSIUS[n], seed);
			case "howl" -> bridge.renderOneShot(NativeBridge.SHOT_HOWL, 0, seed);
			case "brush" -> bridge.renderOneShot(NativeBridge.SHOT_BRUSH, n, seed);
			case "ice" -> bridge.renderOneShot(NativeBridge.SHOT_ICE_SINGING, 0, seed);
			case "settle" -> bridge.renderOneShot(NativeBridge.SHOT_SETTLING_STONE, 0, seed);
			case "flush" -> bridge.renderOneShot(NativeBridge.SHOT_FLUSH, 0, seed);
			case "buzz" -> bridge.renderOneShot(NativeBridge.SHOT_BUZZ, n, seed);
			default -> otherwise;
		};
	}

	/**
	 * A strike or break ({@code <contact>_<material>}, see {@link Layers}), or null. Taps at 3 m/s (a swung tool),
	 * blocks set down at 0.8, bodies landing at 5 (a fall of about 1.3 m), heels at 1.
	 */
	private static short @org.jspecify.annotations.Nullable [] contact(NativeBridge bridge, String model, int variant) {
		int split = model.indexOf('_');
		if (split < 0) return null;
		int contact = java.util.Arrays.asList(Layers.CONTACTS).indexOf(model.substring(0, split));
		int material = java.util.Arrays.asList(Materials.NAMES).indexOf(model.substring(split + 1));
		if (contact < 0 || material < 0) return null;
		float speed = switch (contact) {
			case NativeBridge.CONTACT_PLACE -> 0.8f;
			case NativeBridge.CONTACT_LAND -> 5f;
			case NativeBridge.CONTACT_STEP -> 1f;
			default -> 3f;
		};
		return bridge.renderContact(contact, material, speed, 0xC0A7L * (contact + 1) + 97L * material + variant);
	}

	/** Whether {@code path}, as the buffer library asks for it, is one Foley generates. */
	public static boolean isGeneratedPath(Identifier path) {
		return path.getNamespace().equals("foley") && path.getPath().startsWith("sounds/" + PREFIX);
	}

	/**
	 * The buffer for {@code path}, as the buffer library asks for it ({@code foley:sounds/generated/drip_3.ogg}), or
	 * null if it isn't one of Foley's.
	 */
	public static @Nullable SoundBuffer render(Identifier path) {
		NativeBridge bridge = GeneratedSounds.bridge;
		String name = path.getPath();
		if (bridge == null || !path.getNamespace().equals("foley") || !name.startsWith("sounds/" + PREFIX)) return null;
		name = name.substring(("sounds/" + PREFIX).length()).replace(".ogg", "");
		int split = name.lastIndexOf('_');
		String model = name.substring(0, split);
		int variant = Integer.parseInt(name.substring(split + 1));
		// Another mod's voiced creature: voice_custom_<index>_<archetype>_<variant>.
		if (model.startsWith("voice_custom_")) {
			String[] parts = model.substring("voice_custom_".length()).split("_");
			if (parts.length == 2) {
				int archetype = CustomVoices.archetype(parts[1]);
				if (archetype < 0) return null;
				try {
					return AddonSounds.toBuffer(BedSound.FORMAT, CustomVoices.render(bridge,
							Integer.parseInt(parts[0]), archetype, variant));
				} catch (NumberFormatException e) {
					return null;
				}
			}
			return null;
		}
		// Another mod's material: custom_<index>_<contact>_<variant>.
		if (model.startsWith("custom_")) {
			String[] parts = model.substring("custom_".length()).split("_");
			if (parts.length == 2) {
				try {
					return AddonSounds.toBuffer(BedSound.FORMAT, CustomMaterials.render(bridge,
							Integer.parseInt(parts[0]), Integer.parseInt(parts[1]), variant));
				} catch (NumberFormatException e) {
					return null;
				}
			}
			return null;
		}
		short[] pcm = switch (model) {
			// Drips into water trap bubbles of about 0.6 to 3 mm, spread over the variants.
			case "drip" -> bridge.renderOneShot(NativeBridge.SHOT_DRIP_INTO_WATER,
					0.6f * (float) Math.pow(5, variant / 15.0), 0x5EED + variant);
			// Drops falling from a stalactite are 2 to 2.8 mm across their radius (about the capillary length).
			case "drip_stone" -> bridge.renderOneShot(NativeBridge.SHOT_DRIP_ON_STONE, 2f + 0.8f * variant / 15f,
					0xD121 + variant);
			case "drip_lava" -> bridge.renderOneShot(NativeBridge.SHOT_LAVA_DRIP, 1f, 0x1A7A + variant);
			default -> life(bridge, model, variant, contact(bridge, model, variant));
		};
		if (pcm == null) return null;
		ByteBuffer data = ByteBuffer.allocateDirect(pcm.length * 2).order(ByteOrder.LITTLE_ENDIAN);
		data.asShortBuffer().put(pcm);
		return new SoundBuffer(data, BedSound.FORMAT);
	}
}
