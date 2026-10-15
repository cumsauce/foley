package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.api.FoleyVoices;
import io.github.cumsauce.foley.api.VoiceDescriptor;
import io.github.cumsauce.foley.natives.NativeBridge;
import java.util.ArrayList;
import java.util.IdentityHashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.entity.EntityType;
import org.jspecify.annotations.Nullable;

/**
 * Other mods' creatures (F3): a {@link VoiceDescriptor} under an {@link EntityType}, with the game's call names mapped
 * onto Foley's five archetypes. {@link Voices} asks here for entities its own table doesn't cover, and plays the
 * result as a generated one-shot under {@code foley:generated/voice_custom_<index>_<archetype>_<variant>}, the game's
 * volume and pitch kept, the player's setting ("Addons' Creatures", off by default) gating it.
 */
public final class CustomVoices {
	/** The five archetypes, in the native order. */
	static final List<String> ARCHETYPES = List.of("ambient", "hurt", "death", "growl", "breath");

	private static final Map<EntityType<?>, Defined> BY_TYPE = new IdentityHashMap<>();
	private static final Map<String, Defined> BY_NAME = new LinkedHashMap<>();
	private static boolean frozen;

	/** A creature: its voice, its call map, and which slot its generated names use. */
	record Defined(VoiceDescriptor voice, Map<String, String> calls, int index, String origin) {}

	/** A game sound parsed as one of the custom creatures'. */
	record Parsed(Defined creature, String archetype, float pitch) {}

	private CustomVoices() {}

	/** Registration, as one addon sees it: every refusal names it. */
	static FoleyVoices api(String origin) {
		return new FoleyVoices() {
			@Override
			public void register(EntityType<?> type, VoiceDescriptor voice, Map<String, String> calls) {
				if (frozen)
					throw new IllegalStateException(origin + " voiced " + type + " after its entrypoint returned");
				if (voice == null || calls == null)
					throw new IllegalArgumentException(origin + " needs a voice and a call map");
				for (Map.Entry<String, String> call : calls.entrySet()) {
					if (!ARCHETYPES.contains(call.getValue()))
						throw new IllegalArgumentException("unknown archetype \"" + call.getValue() + "\" ("
								+ origin + ", " + type + ")");
				}
				if (BY_TYPE.containsKey(type))
					throw new IllegalArgumentException(type + " is already voiced by " + BY_TYPE.get(type).origin());
				Defined defined = new Defined(voice, Map.copyOf(calls), BY_TYPE.size(), origin);
				BY_TYPE.put(type, defined);
				BY_NAME.put(BuiltInRegistries.ENTITY_TYPE.getKey(type).getPath(), defined);
			}
		};
	}

	/** Closes registration: every entrypoint has returned. */
	static void freeze() {
		frozen = true;
		FoleyClient.LOGGER.info("Foley: {} custom creature(s) voiced", BY_TYPE.size());
	}

	/** Whether {@code name} (an entity's, as its sound events name it) is one of the custom creatures'. */
	static boolean voiced(String name) {
		return BY_NAME.containsKey(name);
	}

	/**
	 * The game sound {@code entity.<name>.<call>} as a custom creature's, its variant suffixes and babies handled as
	 * the game's own are, or null if it isn't one of the custom creatures' calls.
	 */
	static @Nullable Parsed parse(String name, String call) {
		float pitch = 1f;
		Defined defined = BY_NAME.get(name);
		if (defined == null) {
			int split = name.lastIndexOf('_');
			if (split < 0) return null;
			Float variant = Voices.VARIANTS.get(name.substring(split + 1));
			if (variant == null) return null;
			defined = BY_NAME.get(name.substring(0, split));
			if (defined == null) return null;
			pitch = variant;
		}
		String archetype = defined.calls().get(call);
		if (archetype == null && call.endsWith("_baby")) {
			archetype = defined.calls().get(call.substring(0, call.length() - "_baby".length()));
			pitch *= 1.3f;
		}
		if (archetype == null) return null;
		return new Parsed(defined, archetype, pitch);
	}

	/** The event name for creature {@code index}'s {@code archetype}. */
	static String eventName(int index, String archetype) {
		return "voice_custom_" + index + "_" + archetype;
	}

	/** An archetype's native ordinal, or −1. */
	static int archetype(String archetype) {
		return ARCHETYPES.indexOf(archetype);
	}

	/** Renders one variant of creature {@code index}'s {@code archetype}, or null if there is no such creature. */
	public static short @Nullable [] render(NativeBridge bridge, int index, int archetype, int variant) {
		for (Defined defined : BY_TYPE.values()) {
			if (defined.index() != index) continue;
			return bridge.renderCreature(defined.voice().toFloats(), archetype,
					0x701CEL * (index + 1) + 31L * archetype + variant);
		}
		return null;
	}

	/** The pack's creatures, read at a resource reload: entity id to voice and call map. */
	public static void fromPacks(String namespace, Map<String, Entry> creatures) {
		for (Map.Entry<String, Entry> creature : creatures.entrySet()) {
			Identifier entityId = Identifier.tryParse(creature.getKey());
			if (entityId == null || !ARCHETYPES.containsAll(creature.getValue().calls().values())) continue;
			EntityType<?> type = BuiltInRegistries.ENTITY_TYPE.getValue(entityId);
			Defined defined = new Defined(creature.getValue().voice(), creature.getValue().calls(), BY_TYPE.size(),
					namespace);
			BY_TYPE.putIfAbsent(type, defined);
			BY_NAME.putIfAbsent(entityId.getPath(), defined);
		}
	}

	/** One entry of a pack's {@code foley/voices.json}: its voice and call map. */
	public record Entry(VoiceDescriptor voice, Map<String, String> calls) {}
}
