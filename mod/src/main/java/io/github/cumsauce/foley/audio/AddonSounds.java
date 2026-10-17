package io.github.cumsauce.foley.audio;

import com.mojang.blaze3d.audio.SoundBuffer;
import com.google.gson.JsonObject;
import com.mojang.serialization.JsonOps;
import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.api.FoleyEntrypoint;
import io.github.cumsauce.foley.api.FoleySounds;
import io.github.cumsauce.foley.api.SoundCodecs;
import io.github.cumsauce.foley.api.SoundModel;
import io.github.cumsauce.foley.natives.NativeBridge;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.atomic.AtomicInteger;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.AbstractSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.client.sounds.SoundManager;
import net.minecraft.client.sounds.WeighedSoundEvents;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundSource;
import net.minecraft.util.GsonHelper;
import net.minecraft.util.valueproviders.ConstantFloat;
import org.jspecify.annotations.Nullable;

/**
 * Sounds other mods define (M0's delivery, generalized past Foley's own namespace): a {@link SoundModel} under an id,
 * registered from a {@code foley} entrypoint or read from {@code assets/<ns>/foley/sounds.json}, rendered by the same
 * path as Foley's own one-shots — off the asking thread, into the game's buffer cache, played from its pool of static
 * sources. Variant {@code n} of {@code <ns>:<name>} lives at {@code <ns>:foley/<name>_<n>}, which the buffer library
 * asks for as {@code sounds/foley/<name>_<n>.ogg}.
 */
public final class AddonSounds {
	private static @Nullable NativeBridge bridge;
	private static final Map<Identifier, Entry> CODE = new ConcurrentHashMap<>();
	private static volatile Map<Identifier, Entry> PACKS = Map.of();
	private static boolean frozen;
	private static final AtomicInteger RENDERED = new AtomicInteger();

	/** A definition: what to render, how many variants, and who made it (for errors). */
	record Entry(SoundModel model, int variants, String origin) {}

	private AddonSounds() {}

	/** The native synthesis is up. Called before any entrypoint. */
	public static void use(NativeBridge bridge) {
		AddonSounds.bridge = bridge;
	}

	/**
	 * Calls every other mod that integrates with Foley (its {@code foley} entrypoints), one by one, each failure
	 * logged with its mod's name and skipped: a broken addon loses its sounds, never the game. Registration closes
	 * when the last returns. Client init, once the synthesis and the settings are up.
	 */
	public static void start(NativeBridge bridge) {
		use(bridge);
		AddonSoundFiles.register();
		FoleyProvider.ready(new FoleyApiImpl(""));
		var entrypoints = FabricLoader.getInstance().getEntrypointContainers("foley", FoleyEntrypoint.class);
		for (var container : entrypoints) {
			String addon = container.getProvider().getMetadata().getId();
			try {
				container.getEntrypoint().onFoleyReady(new FoleyApiImpl(addon));
			} catch (Throwable t) {
				FoleyClient.LOGGER.error("Foley: addon {} failed; its sounds are skipped", addon, t);
			}
		}
		freeze();
		CustomMaterials.freeze();
		CustomVoices.freeze();
		if (!entrypoints.isEmpty()) FoleyClient.LOGGER.info("Foley: {} addon(s) integrated", entrypoints.size());
	}

	/** Defines {@code id} for {@code origin} (an addon's mod id), while entrypoints run. */
	static void register(String origin, Identifier id, SoundModel model, int variants) {
		if (frozen) throw new IllegalStateException(origin + " registered " + id + " after its entrypoint returned");
		if (!origin.equals(id.getNamespace()))
			throw new IllegalArgumentException(origin + " may not define " + id + ": a sound's namespace is its owner's");
		if (!(1 <= variants && variants <= 64))
			throw new IllegalArgumentException("variants must be 1 to 64, not " + variants + " (" + origin + ", " + id + ")");
		if (model instanceof SoundModel.Voice voice) {
			if (Voices.MOBS.indexOf(voice.mob()) < 0)
				throw new IllegalArgumentException("unknown mob \"" + voice.mob() + "\" (" + origin + ", " + id + ")");
			if (Voices.CALLS.indexOf(voice.call()) < 0)
				throw new IllegalArgumentException("unknown call \"" + voice.call() + "\" (" + origin + ", " + id + ")");
		}
		Entry was = CODE.putIfAbsent(id, new Entry(model, variants, origin));
		if (was != null) throw new IllegalArgumentException(id + " is already defined by " + was.origin());
	}

	/** Closes registration: every entrypoint has returned. */
	static void freeze() {
		frozen = true;
	}

	/** Takes the pack definitions read at a resource reload, replacing the last. Code's own win; clashes are logged. */
	static void fromPacks(Map<Identifier, Entry> packs) {
		Map<Identifier, Entry> kept = new java.util.HashMap<>(packs);
		for (Identifier clash : packs.keySet().stream().filter(CODE::containsKey).toList()) {
			FoleyClient.LOGGER.error("Foley: pack sound {} is already defined in code; the code's is kept", clash);
			kept.remove(clash);
		}
		PACKS = Map.copyOf(kept);
	}

	static @Nullable Entry find(Identifier id) {
		Entry pack = PACKS.get(id);
		return pack != null ? pack : CODE.get(id);
	}

	/** Whether {@code location} (a sound's own name, {@code <ns>:foley/<name>_<n>}) is one an addon defines. */
	public static boolean isGenerated(Identifier location) {
		return location.getPath().startsWith("foley/");
	}

	/** A buffer-library path taken apart: the sound it names, and which variant. */
	private record Parsed(Identifier id, int variant) {}

	/** Whether {@code path} (as the buffer library asks for it) names a defined addon variant. */
	public static boolean isGeneratedPath(Identifier path) {
		Parsed parsed = parse(path);
		return parsed != null && find(parsed.id()) != null;
	}

	/** The (id, variant) a buffer-library path names, or null if it isn't shaped like one. */
	private static @Nullable Parsed parse(Identifier path) {
		String name = path.getPath();
		String prefix = "sounds/foley/";
		if (!name.startsWith(prefix) || !name.endsWith(".ogg")) return null;
		name = name.substring(prefix.length(), name.length() - ".ogg".length());
		int split = name.lastIndexOf('_');
		if (split < 0) return null;
		int variant;
		try {
			variant = Integer.parseInt(name.substring(split + 1));
		} catch (NumberFormatException e) {
			return null;
		}
		return new Parsed(Identifier.fromNamespaceAndPath(path.getNamespace(), name.substring(0, split)), variant);
	}

	/**
	 * The buffer for {@code path}, as the buffer library asks for it, or null if it isn't a defined addon sound.
	 * Called off the client thread, like a file load.
	 */
	public static @Nullable SoundBuffer render(Identifier path) {
		NativeBridge bridge = AddonSounds.bridge;
		Parsed parsed = parse(path);
		if (bridge == null || parsed == null) return null;
		Entry entry = find(parsed.id());
		if (entry == null || parsed.variant() >= entry.variants()) return null;
		short[] pcm = render(bridge, entry.model(), seed(parsed.id(), parsed.variant()));
		RENDERED.incrementAndGet();
		return toBuffer(BedSound.FORMAT, pcm);
	}

	/** {@code pcm} (48 kHz mono, 16-bit) as a sound buffer, or null for nothing (silence included). */
	static @Nullable SoundBuffer toBuffer(javax.sound.sampled.AudioFormat format, short @Nullable [] pcm) {
		if (pcm == null || pcm.length == 0) return null;
		ByteBuffer data = ByteBuffer.allocateDirect(pcm.length * 2).order(ByteOrder.LITTLE_ENDIAN);
		data.asShortBuffer().put(pcm);
		return new SoundBuffer(data, format);
	}

	/** One model, one seed: the whole catalogue onto the native calls. */
	private static short[] render(NativeBridge bridge, SoundModel model, long seed) {
		return switch (model) {
			case SoundModel.DripIntoWater m -> bridge.renderOneShot(NativeBridge.SHOT_DRIP_INTO_WATER, m.radiusMm(),
					seed);
			case SoundModel.DripOnStone m -> bridge.renderOneShot(NativeBridge.SHOT_DRIP_ON_STONE, m.radiusMm(), seed);
			case SoundModel.LavaDrip m -> bridge.renderOneShot(NativeBridge.SHOT_LAVA_DRIP, 1f, seed);
			case SoundModel.Crickets m -> bridge.renderOneShot(NativeBridge.SHOT_CRICKETS, m.celsius(), seed);
			case SoundModel.Birdsong m -> bridge.renderOneShot(NativeBridge.SHOT_BIRDSONG, m.species(), seed);
			case SoundModel.Frog m -> bridge.renderOneShot(NativeBridge.SHOT_FROG, m.lengthCm(), seed);
			case SoundModel.Surf m -> bridge.renderOneShot(NativeBridge.SHOT_SURF, m.waveHeightM(), seed);
			case SoundModel.Woodpecker m -> bridge.renderOneShot(NativeBridge.SHOT_WOODPECKER, 0f, seed);
			case SoundModel.Owl m -> bridge.renderOneShot(NativeBridge.SHOT_OWL, 0f, seed);
			case SoundModel.Cicadas m -> bridge.renderOneShot(NativeBridge.SHOT_CICADAS, m.celsius(), seed);
			case SoundModel.Howl m -> bridge.renderOneShot(NativeBridge.SHOT_HOWL, 0f, seed);
			case SoundModel.Brush m -> bridge.renderOneShot(NativeBridge.SHOT_BRUSH, m.plant().ordinal(), seed);
			case SoundModel.IceSinging m -> bridge.renderOneShot(NativeBridge.SHOT_ICE_SINGING, 0f, seed);
			case SoundModel.SettlingStone m -> bridge.renderOneShot(NativeBridge.SHOT_SETTLING_STONE, 0f, seed);
			case SoundModel.Flush m -> bridge.renderOneShot(NativeBridge.SHOT_FLUSH, 0f, seed);
			case SoundModel.Buzz m -> bridge.renderOneShot(NativeBridge.SHOT_BUZZ, m.insect().ordinal(), seed);
			case SoundModel.SculkRing m -> bridge.renderOneShot(NativeBridge.SHOT_SCULK_RING, m.q(), seed);
			case SoundModel.Bell m -> bridge.renderOneShot(NativeBridge.SHOT_BELL, m.q(), seed);
			// The primitives (F2), composed into sounds of the addon's own invention.
			case SoundModel.Bubble m -> bridge.renderPrimitive(NativeBridge.PRIMITIVE_BUBBLE,
					new float[] {m.radiusMm()}, seed);
			case SoundModel.NoiseBurst m -> bridge.renderPrimitive(NativeBridge.PRIMITIVE_NOISE,
					new float[] {m.centreHz(), m.q(), m.seconds(), m.gain()}, seed);
			case SoundModel.Tone m -> bridge.renderPrimitive(NativeBridge.PRIMITIVE_TONE,
					new float[] {m.centreHz(), m.targetHz(), m.seconds(), m.gain()}, seed);
			case SoundModel.Gust m -> bridge.renderPrimitive(NativeBridge.PRIMITIVE_GUST,
					new float[] {m.speed(), m.seconds(), m.gain()}, seed);
			case SoundModel.Contact m -> bridge.renderContact(m.kind().ordinal(), m.material().ordinal(), m.speed(),
					seed);
			case SoundModel.Voice m -> bridge.renderVoice(Voices.MOBS.indexOf(m.mob()), Voices.CALLS.indexOf(m.call()),
					seed);
		};
	}

	/** A stable seed for one variant of one sound, so a variant stays itself across sessions. */
	private static long seed(Identifier id, int variant) {
		return id.toString().hashCode() * 0x9E3779B97F4A7C15L + 0x1A7AL * variant;
	}

	/**
	 * Plays defined sound {@code id} at ({@code x}, {@code y}, {@code z}), unless bypassed or past this tick's share.
	 * Client thread.
	 */
	static boolean play(Identifier id, double x, double y, double z, SoundSource source, float volume, float pitch) {
		Entry entry = find(id);
		if (entry == null) throw new IllegalArgumentException(id + " is not a defined sound");
		if (!FoleyClient.wants(config -> true) || !FoleyClient.oneShot()) return false;
		int variant = SoundInstance.createUnseededRandom().nextInt(entry.variants());
		Identifier location = Identifier.fromNamespaceAndPath(id.getNamespace(), "foley/" + id.getPath() + "_" + variant);
		Minecraft.getInstance().getSoundManager()
				.play(new Instance(id, location, source, volume * FoleyClient.gain(), pitch, x, y, z));
		return true;
	}

	/** How many addon buffers have been rendered. For the client gametest. */
	public static int rendered() {
		return RENDERED.get();
	}

	/**
	 * A playable sound for {@code id} (its event's name) that resolves itself to {@code variant} (a generated buffer
	 * path), needing no sounds.json: Foley's own layers play generated names this way too.
	 */
	public static SoundInstance instance(Identifier id, Identifier variant, SoundSource source, float volume,
			float pitch, double x, double y, double z) {
		return new Instance(id, variant, source, volume, pitch, x, y, z);
	}

	/** One entry of a pack's {@code assets/<ns>/foley/sounds.json}: its model and variant count. */
	static Entry parseEntry(JsonObject entry, String where) {
		int variants = entry.has("variants") ? GsonHelper.getAsInt(entry, "variants") : FoleySounds.DEFAULT_VARIANTS;
		if (!(1 <= variants && variants <= 64))
			throw new IllegalArgumentException("variants must be 1 to 64, not " + variants + " (" + where + ")");
		SoundModel model = SoundCodecs.CODEC.parse(JsonOps.INSTANCE, entry).getOrThrow();
		return new Entry(model, variants, where);
	}

	/**
	 * A played addon sound, resolved without sounds.json: its event is the registered id (for nothing more than
	 * naming), its sound one variant of it.
	 */
	private static final class Instance extends AbstractSoundInstance {
		private final Identifier variant;

		Instance(Identifier id, Identifier variant, SoundSource source, float volume, float pitch, double x, double y,
				double z) {
			super(id, source, SoundInstance.createUnseededRandom());
			this.variant = variant;
			this.volume = volume;
			this.pitch = pitch;
			this.x = x;
			this.y = y;
			this.z = z;
		}

		@Override
		public WeighedSoundEvents getOrResolve(SoundManager manager) {
			if (this.soundEvent == null) {
				WeighedSoundEvents events = new WeighedSoundEvents(this.identifier, null);
				events.addSound(new net.minecraft.client.resources.sounds.Sound(variant, ConstantFloat.of(1f),
						ConstantFloat.of(1f), 1, net.minecraft.client.resources.sounds.Sound.Type.FILE, false, false,
						16));
				this.soundEvent = events;
				this.sound = events.getSound(this.random);
			}
			return this.soundEvent;
		}
	}
}
