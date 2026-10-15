package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.mixin.AbstractSoundInstanceAccessor;
import java.util.List;
import java.util.Map;
import java.util.concurrent.atomic.AtomicInteger;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.Sound;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import net.minecraft.util.RandomSource;
import net.minecraft.util.valueproviders.ConstantFloat;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * Procedural mob voices (M6, remade in M7, more in M8 and M9), switched on in the settings group by group (off by
 * default). The game's calls for a mob ({@link VoiceTable#GAME}), in all their variants ({@code entity.wolf_big.hurt},
 * {@code entity.cat.ambient_baby}), are replaced by Foley's, keeping the game's volume and pitch (a baby's is higher;
 * bigger variants a little lower). A zombie close by strains, louder; {@link Breathing} adds its breath. A bee's buzz
 * loops, so rather than replaced, the game's own looping instance is pointed at Foley's sound ({@link #redirect}). A
 * parrot imitating a mob does it in Foley's voice for that mob.
 */
public final class Voices {
	public static final List<String> GROUPS = withAddon(VoiceTable.GROUPS);
	public static final List<String> MOBS = VoiceTable.MOBS;
	public static final List<String> CALLS = VoiceTable.CALLS;
	/** The settings group the custom creatures share, appended after the game mobs'. */
	public static final int ADDON_GROUP = VoiceTable.GROUPS.size();

	private static List<String> withAddon(List<String> base) {
		List<String> groups = new java.util.ArrayList<>(base);
		groups.add("addon");
		return List.copyOf(groups);
	}
	/** Variants and how much higher they are. */
	static final Map<String, Float> VARIANTS = Map.ofEntries(Map.entry("moody", 0.95f),
			Map.entry("big", 0.85f), Map.entry("mini", 1.25f), Map.entry("picky", 1.05f), Map.entry("angry", 0.95f),
			Map.entry("cute", 1.15f), Map.entry("grumpy", 0.9f), Map.entry("puglin", 1.1f), Map.entry("sad", 1.05f),
			Map.entry("royal", 0.95f));
	/** A zombie's ambient call within this many blocks is its strained, louder one. */
	private static final double NEAR = 8;
	private static final RandomSource RANDOM = RandomSource.create();
	private static final AtomicInteger REDIRECTED = new AtomicInteger();

	private Voices() {}

	private static final Map<String, List<String>> CALLS_OF = new java.util.concurrent.ConcurrentHashMap<>();

	/** The calls Foley makes for native mob {@code mob}. */
	public static List<String> callsOf(String mob) {
		return CALLS_OF.computeIfAbsent(mob, Voices::findCalls);
	}

	private static List<String> findCalls(String mob) {
		var calls = new java.util.ArrayList<String>();
		VoiceTable.GAME.forEach((game, table) -> table.forEach((call, target) -> {
			String m = target.contains(":") ? target.substring(0, target.indexOf(':')) : game;
			String c = target.contains(":") ? target.substring(target.indexOf(':') + 1) : target;
			if (m.equals(mob) && !calls.contains(c)) calls.add(c);
		}));
		calls.addAll(VoiceTable.EXTRA.getOrDefault(mob, List.of()));
		return List.copyOf(calls);
	}

	/** The settings group native mob {@code mob} belongs to, as a bit index, or -1. */
	public static int group(String mob) {
		return GROUPS.indexOf(VoiceTable.GROUP_OF.getOrDefault(mob, ""));
	}

	/** Whether Foley voices native mob {@code mob} now. */
	public static boolean voiced(String mob) {
		int group = group(mob);
		return group >= 0 && FoleyClient.wants(config -> config.voice(group));
	}

	/** Game sounds pointed at Foley's (bee buzzes). For the client gametest. */
	public static int redirected() {
		return REDIRECTED.get();
	}

	/** A game sound as Foley voices it: the native mob and call, and how much higher its variant is. */
	record Parsed(String mob, String call, float pitch) {}

	static @Nullable Parsed parse(Identifier id) {
		if (!id.getNamespace().equals("minecraft") || !id.getPath().startsWith("entity.")) return null;
		String path = id.getPath().substring("entity.".length());
		int dot = path.indexOf('.');
		if (dot < 0) return null;
		String name = path.substring(0, dot);
		String call = path.substring(dot + 1);
		float pitch = 1f;
		Map<String, String> table = VoiceTable.GAME.get(name);
		if (table == null) {
			int split = name.lastIndexOf('_');
			if (split < 0) return null;
			Float variant = VARIANTS.get(name.substring(split + 1));
			table = VoiceTable.GAME.get(name.substring(0, split));
			if (variant == null || table == null) return null;
			name = name.substring(0, split);
			pitch = variant;
		}
		String target = table.get(call);
		if (target == null && call.endsWith("_baby")) {
			target = table.get(call.substring(0, call.length() - "_baby".length()));
			pitch *= 1.3f;
		}
		if (target == null) return null;
		int colon = target.indexOf(':');
		return colon < 0 ? new Parsed(name, target, pitch)
				: new Parsed(target.substring(0, colon), target.substring(colon + 1), pitch);
	}

	/** Whether to skip {@code sound}, having played Foley's voice for it instead. */
	public static boolean replace(SoundInstance sound) {
		Identifier id = sound.getIdentifier();
		if (id.getPath().startsWith("entity.parrot.imitate.")) return imitate(sound, id);
		// Another mod's creature first: its sounds aren't in the game's voice table.
		if (id.getNamespace().equals("minecraft") && id.getPath().startsWith("entity.")) {
			String path = id.getPath().substring("entity.".length());
			int dot = path.indexOf('.');
			if (dot > 0) {
				CustomVoices.Parsed creature = CustomVoices.parse(path.substring(0, dot), path.substring(dot + 1));
				if (creature != null) return replaceCustom(sound, creature);
			}
		}
		Parsed parsed = parse(id);
		if (parsed == null || parsed.call.startsWith("loop") || !voiced(parsed.mob)) return false;
		// Past the cap, the game's own call plays.
		if (!FoleyClient.oneShot()) return false;
		float volume = 1f;
		float pitch = 1f;
		if (sound instanceof AbstractSoundInstanceAccessor own) {
			volume = own.foley$volume();
			pitch = own.foley$pitch();
		}
		// Each mob its own voice, the same call to call and session to session, in its own habits.
		Individuals.Voice own = Individuals.of(parsed.mob, sound.getX(), sound.getY(), sound.getZ(), pitch);
		if (own != null) pitch = own.pitch();
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
		play(minecraft, parsed.mob, call, sound.getSource(), volume, pitch, sound.getX(), sound.getY(), sound.getZ(),
				own);
		FoleyClient.countReplaced();
		return true;
	}

	/** A custom creature's game sound, spoken by Foley's creature synthesis: the game's volume and pitch kept. */
	private static boolean replaceCustom(SoundInstance sound, CustomVoices.Parsed creature) {
		if (!FoleyClient.wants(config -> config.voice(ADDON_GROUP)) || !FoleyClient.oneShot()) return false;
		float volume = 1f;
		float pitch = 1f;
		if (sound instanceof AbstractSoundInstanceAccessor own) {
			volume = own.foley$volume();
			pitch = own.foley$pitch();
		}
		String name = CustomVoices.eventName(creature.creature().index(), creature.archetype());
		int variant = RANDOM.nextInt(8);
		Minecraft.getInstance().getSoundManager().play(AddonSounds.instance(
				Sounds.event(name).location(),
				Identifier.fromNamespaceAndPath("foley", GeneratedSounds.PREFIX + name + "_" + variant),
				sound.getSource(), volume * FoleyClient.gain(), Math.clamp(pitch * creature.pitch(), 0.5f, 2f),
				sound.getX(), sound.getY(), sound.getZ()));
		FoleyClient.countReplaced();
		return true;
	}

	/** A parrot imitating a mob Foley voices, in that voice, higher. */
	private static boolean imitate(SoundInstance sound, Identifier id) {
		if (!voiced("parrot")) return false;
		String mob = id.getPath().substring("entity.parrot.imitate.".length());
		if (!MOBS.contains(mob)) return false;
		List<String> calls = callsOf(mob);
		String call = calls.contains("ambient") ? "ambient" : calls.isEmpty() ? null : calls.getFirst();
		if (call == null) return false;
		float pitch = sound instanceof AbstractSoundInstanceAccessor own ? own.foley$pitch() : 1f;
		play(Minecraft.getInstance(), mob, call, sound.getSource(), 0.7f, pitch * 1.5f, sound.getX(), sound.getY(),
				sound.getZ(), null);
		FoleyClient.countReplaced();
		return true;
	}

	/**
	 * Foley's sound for a looping game sound it voices (a bee's buzz), to put in place of the one the game resolved,
	 * or null. The game's instance keeps doing the rest: looping, following the bee, rising in pitch as it flies.
	 */
	public static @Nullable Sound redirect(SoundInstance sound) {
		Parsed parsed = parse(sound.getIdentifier());
		if (parsed == null || !parsed.call.startsWith("loop") || !voiced(parsed.mob)) return null;
		REDIRECTED.incrementAndGet();
		Identifier generated = Identifier.fromNamespaceAndPath("foley", GeneratedSounds.PREFIX + "voice_" + parsed.mob
				+ "_" + parsed.call + "_" + RANDOM.nextInt(4));
		return new Sound(generated, ConstantFloat.of(1f), ConstantFloat.of(1f), 1, Sound.Type.FILE, false, false, 16);
	}

	static SoundEvent event(String mob, String call) {
		return Sounds.event("voice_" + mob + "_" + call);
	}

	/** Plays a call; {@code own}, if there is one, picks which of its renditions rather than leaving it to chance. */
	private static void play(Minecraft minecraft, String mob, String call, SoundSource source, float volume,
			float pitch, double x, double y, double z, Individuals.@Nullable Voice own) {
		minecraft.getSoundManager().play(new SimpleSoundInstance(event(mob, call), source, volume * FoleyClient.gain(),
				Math.clamp(pitch, 0.5f, 2f), SoundInstance.createUnseededRandom(), x, y, z) {
			@Override
			public net.minecraft.client.sounds.WeighedSoundEvents getOrResolve(
					net.minecraft.client.sounds.SoundManager manager) {
				var events = super.getOrResolve(manager);
				if (own != null && sound != null && events != null) {
					sound = rendition(sound, own.rendition(events.getWeight()));
				}
				return events;
			}
		});
	}

	/** {@code sound} (a generated rendition, {@code …_<n>}) as its rendition {@code n}, keeping the rest. */
	static Sound rendition(Sound sound, int n) {
		String path = sound.getLocation().getPath();
		int split = path.lastIndexOf('_');
		if (split < 0 || !path.startsWith(GeneratedSounds.PREFIX)) return sound;
		Identifier chosen = Identifier.fromNamespaceAndPath(sound.getLocation().getNamespace(),
				path.substring(0, split + 1) + n);
		return new Sound(chosen, sound.getVolume(), sound.getPitch(), sound.getWeight(), sound.getType(),
				sound.shouldStream(), sound.shouldPreload(), sound.getAttenuationDistance());
	}
}
