package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.mixin.AbstractSoundInstanceAccessor;
import java.util.List;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * Procedural mob voices (M6, remade in M7), each switched on in the settings (off by default), group by group:
 * zombies (with husks, drowned and zombie villagers), skeletons (strays, bogged), spiders, and so on. A mob's calls,
 * in all their variants ({@code entity.wolf_big.hurt}, {@code entity.cat.ambient_baby}), are replaced by Foley's,
 * keeping the game's volume and pitch (a baby's is higher; bigger variants a little lower). A zombie close to the
 * listener strains, louder; {@link Breathing} adds its breath.
 */
public final class Voices {
	/** The settings' groups, in the order of their bits. */
	public static final List<String> GROUPS = List.of("cow", "pig", "sheep", "chicken", "villager", "zombie", "wolf",
			"cat", "skeleton", "spider", "creeper", "bat");
	/** The native mobs, in the C ABI's order. */
	public static final List<String> MOBS = List.of("cow", "pig", "sheep", "chicken", "villager", "zombie", "wolf",
			"cat", "husk", "drowned", "zombie_villager", "skeleton", "stray", "bogged", "spider", "cave_spider",
			"creeper", "bat");
	/** The native calls, in the C ABI's order. */
	public static final List<String> CALLS = List.of("ambient", "hurt", "death", "growl", "pant", "whine", "hiss",
			"purr", "beg", "primed", "ambient_water", "hurt_water", "death_water", "breath", "breath_heavy",
			"ambient_near");
	/** Each mob's calls Foley makes (the rest stay the game's). */
	public static final Map<String, List<String>> CALLS_OF = Map.ofEntries(
			Map.entry("wolf", List.of("ambient", "hurt", "death", "growl", "pant", "whine")),
			Map.entry("cat", List.of("ambient", "hurt", "death", "hiss", "purr", "beg")),
			Map.entry("creeper", List.of("primed", "hurt", "death")),
			Map.entry("zombie", List.of("ambient", "hurt", "death", "breath", "breath_heavy", "ambient_near")),
			Map.entry("husk", List.of("ambient", "hurt", "death", "breath", "breath_heavy", "ambient_near")),
			Map.entry("zombie_villager", List.of("ambient", "hurt", "death", "breath", "breath_heavy",
					"ambient_near")),
			Map.entry("drowned", List.of("ambient", "hurt", "death", "ambient_water", "hurt_water", "death_water",
					"breath", "breath_heavy", "ambient_near")));
	/** The game's names for calls, where they differ. */
	private static final Map<String, String> GAME_CALLS = Map.of("beg_for_food", "beg", "purreow", "beg",
			"stray_ambient", "ambient");
	/** Variants and how much higher they are. */
	private static final Map<String, Float> VARIANTS = Map.ofEntries(Map.entry("moody", 0.95f),
			Map.entry("big", 0.85f), Map.entry("mini", 1.25f), Map.entry("picky", 1.05f), Map.entry("angry", 0.95f),
			Map.entry("cute", 1.15f), Map.entry("grumpy", 0.9f), Map.entry("puglin", 1.1f), Map.entry("sad", 1.05f),
			Map.entry("royal", 0.95f));
	/** A zombie's ambient call within this many blocks is its strained, louder one. */
	private static final double NEAR = 8;

	private Voices() {}

	/** A mob's calls. */
	public static List<String> callsOf(String mob) {
		return CALLS_OF.getOrDefault(mob, List.of("ambient", "hurt", "death"));
	}

	/** The settings group {@code mob} belongs to. */
	public static int group(String mob) {
		return switch (mob) {
			case "husk", "drowned", "zombie_villager" -> GROUPS.indexOf("zombie");
			case "stray", "bogged" -> GROUPS.indexOf("skeleton");
			case "cave_spider" -> GROUPS.indexOf("spider");
			default -> GROUPS.indexOf(mob);
		};
	}

	/** Whether Foley voices {@code mob} now. */
	public static boolean voiced(String mob) {
		int group = group(mob);
		return group >= 0 && FoleyClient.wants(config -> config.voice(group));
	}

	/** A sound event's mob, pitch for its variant, and call, or null if it's not one Foley voices. */
	record Parsed(String mob, float pitch, String call) {}

	static @Nullable Parsed parse(Identifier id) {
		if (!id.getNamespace().equals("minecraft") || !id.getPath().startsWith("entity.")) return null;
		String[] parts = id.getPath().split("\\.");
		if (parts.length != 3) return null;
		String name = parts[1];
		float pitch = 1f;
		String mob = MOBS.contains(name) ? name : null;
		if (mob == null) {
			int split = name.lastIndexOf('_');
			if (split < 0) return null;
			String base = name.substring(0, split);
			Float variant = VARIANTS.get(name.substring(split + 1));
			if (variant == null || !MOBS.contains(base)) return null;
			mob = base;
			pitch = variant;
		}
		String call = parts[2];
		if (call.endsWith("_baby")) {
			call = call.substring(0, call.length() - "_baby".length());
			pitch *= 1.3f;
		}
		call = GAME_CALLS.getOrDefault(call, call);
		return callsOf(mob).contains(call) ? new Parsed(mob, pitch, call) : null;
	}

	/** Whether to skip {@code sound}, having played Foley's voice for it instead. */
	public static boolean replace(SoundInstance sound) {
		Parsed parsed = parse(sound.getIdentifier());
		if (parsed == null || !voiced(parsed.mob)) return false;
		float volume = 1f;
		float pitch = 1f;
		if (sound instanceof AbstractSoundInstanceAccessor own) {
			volume = own.foley$volume();
			pitch = own.foley$pitch();
		}
		pitch *= parsed.pitch;
		String call = parsed.call;
		Minecraft minecraft = Minecraft.getInstance();
		if (call.equals("ambient") && callsOf(parsed.mob).contains("ambient_near") && minecraft.player != null) {
			double distance = minecraft.player.getEyePosition().distanceTo(new Vec3(sound.getX(), sound.getY(),
					sound.getZ()));
			// Closer, it strains and is louder; further, a little quieter.
			if (distance < NEAR) call = "ambient_near";
			volume *= (float) (0.7 + 0.3 * Math.clamp(1 - distance / 16, 0, 1));
		}
		play(minecraft, parsed.mob, call, sound.getSource(), volume, pitch, sound.getX(), sound.getY(), sound.getZ());
		FoleyClient.countReplaced();
		return true;
	}

	static SoundEvent event(String mob, String call) {
		return SoundEvent.createVariableRangeEvent(Identifier.fromNamespaceAndPath("foley", "voice_" + mob + "_" + call));
	}

	private static void play(Minecraft minecraft, String mob, String call, net.minecraft.sounds.SoundSource source,
			float volume, float pitch, double x, double y, double z) {
		minecraft.getSoundManager().play(new SimpleSoundInstance(event(mob, call), source, volume * FoleyClient.gain(),
				Math.clamp(pitch, 0.5f, 2f), SoundInstance.createUnseededRandom(), x, y, z));
	}
}
