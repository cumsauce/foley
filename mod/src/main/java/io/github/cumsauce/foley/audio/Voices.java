package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.mixin.AbstractSoundInstanceAccessor;
import java.util.List;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;

/**
 * Procedural mob voices (M6), each mob switched on in the settings (off by default): its ambient, hurt and death
 * sounds, in all their variants ({@code entity.wolf_big.hurt}, {@code entity.cat.ambient_baby}, a zombie villager's),
 * replaced by a source–filter voice from the mob's anatomy, keeping the game's volume and pitch (a baby's is higher).
 * Bigger and smaller variants are a little lower or higher.
 */
public final class Voices {
	public static final List<String> MOBS = List.of("cow", "pig", "sheep", "chicken", "villager", "zombie", "wolf",
			"cat");
	public static final List<String> CALLS = List.of("ambient", "hurt", "death");
	private static final Pattern SOUND = Pattern.compile(
			"entity\\.(cow|pig|sheep|chicken|villager|zombie|wolf|cat)(?:_([a-z]+))?\\.(ambient|hurt|death)(_baby)?");
	/** Variants that aren't the same mob, and how much higher the rest are. */
	private static final Map<String, Float> VARIANTS = Map.ofEntries(Map.entry("moody", 0.95f),
			Map.entry("big", 0.85f), Map.entry("mini", 1.25f), Map.entry("picky", 1.05f), Map.entry("villager", 1f),
			Map.entry("angry", 0.95f), Map.entry("cute", 1.15f), Map.entry("grumpy", 0.9f), Map.entry("puglin", 1.1f),
			Map.entry("sad", 1.05f), Map.entry("royal", 0.95f));

	private Voices() {}

	/** Whether to skip {@code sound}, having played Foley's voice for it instead. */
	public static boolean replace(SoundInstance sound) {
		Identifier id = sound.getIdentifier();
		if (!id.getNamespace().equals("minecraft") || !id.getPath().startsWith("entity.")) return false;
		Matcher match = SOUND.matcher(id.getPath());
		if (!match.matches()) return false;
		String variant = match.group(2);
		if (variant != null && !VARIANTS.containsKey(variant)) return false;
		int mob = MOBS.indexOf(match.group(1));
		if (!FoleyClient.wants(config -> config.voice(mob))) return false;
		float volume = 1f;
		float pitch = 1f;
		if (sound instanceof AbstractSoundInstanceAccessor own) {
			volume = own.foley$volume();
			pitch = own.foley$pitch();
		}
		if (variant != null) pitch *= VARIANTS.get(variant);
		if (match.group(4) != null) pitch *= 1.3f;
		SoundEvent event = SoundEvent.createVariableRangeEvent(Identifier.fromNamespaceAndPath("foley",
				"voice_" + match.group(1) + "_" + match.group(3)));
		Minecraft.getInstance().getSoundManager().play(new SimpleSoundInstance(event, sound.getSource(),
				volume * FoleyClient.gain(), Math.clamp(pitch, 0.5f, 2f), SoundInstance.createUnseededRandom(),
				sound.getX(), sound.getY(), sound.getZ()));
		FoleyClient.countReplaced();
		return true;
	}
}
