package io.github.cumsauce.foley.api;

import java.util.Map;
import net.minecraft.world.entity.EntityType;

/**
 * A mod's own creatures, voiced. Describe a creature's voice ({@link VoiceDescriptor}), say which of the game's calls
 * map to which of the five archetypes, and Foley speaks for the creature in the game's sound events — keeping their
 * volume and pitch (babies higher), while the game's subtitles stay. Off until players switch the group on in
 * Foley's settings (under "Addons' Creatures").
 *
 * <p>Registration happens only while a {@link FoleyEntrypoint} runs.
 */
public interface FoleyVoices {
	/** The five calls Foley makes. Values {@code calls} may take. */
	java.util.Set<String> ARCHETYPES = java.util.Set.of("ambient", "hurt", "death", "growl", "breath");

	/**
	 * Voices {@code type}: {@code calls} maps the game's call names (as in the entity's sounds, {@code "ambient"},
	 * {@code "hurt"}, {@code "ambient_water"}, …) onto the archetypes; a game call not mapped, nor its name without a
	 * {@code _baby} suffix mapped, keeps the game's own sound. Throws {@link IllegalStateException} after the
	 * entrypoint, {@link IllegalArgumentException} for an archetype Foley doesn't make or a duplicate creature.
	 */
	void register(EntityType<?> type, VoiceDescriptor voice, Map<String, String> calls);
}
