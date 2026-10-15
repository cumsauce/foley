package io.github.cumsauce.foley.api;

import com.mojang.serialization.Codec;
import com.mojang.serialization.DataResult;
import com.mojang.serialization.DynamicOps;
import com.mojang.serialization.MapCodec;
import com.mojang.serialization.MapLike;
import com.mojang.serialization.RecordBuilder;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import com.mojang.datafixers.util.Pair;
import io.github.cumsauce.foley.api.SoundModel.Birdsong;
import io.github.cumsauce.foley.api.SoundModel.Brush;
import io.github.cumsauce.foley.api.SoundModel.Buzz;
import io.github.cumsauce.foley.api.SoundModel.Cicadas;
import io.github.cumsauce.foley.api.SoundModel.Contact;
import io.github.cumsauce.foley.api.SoundModel.Crickets;
import io.github.cumsauce.foley.api.SoundModel.DripIntoWater;
import io.github.cumsauce.foley.api.SoundModel.DripOnStone;
import io.github.cumsauce.foley.api.SoundModel.Flush;
import io.github.cumsauce.foley.api.SoundModel.Tone;
import io.github.cumsauce.foley.api.SoundModel.NoiseBurst;
import io.github.cumsauce.foley.api.SoundModel.Gust;
import io.github.cumsauce.foley.api.SoundModel.Bubble;
import io.github.cumsauce.foley.api.SoundModel.Frog;
import io.github.cumsauce.foley.api.SoundModel.Howl;
import io.github.cumsauce.foley.api.SoundModel.IceSinging;
import io.github.cumsauce.foley.api.SoundModel.InsectKind;
import io.github.cumsauce.foley.api.SoundModel.LavaDrip;
import io.github.cumsauce.foley.api.SoundModel.Owl;
import io.github.cumsauce.foley.api.SoundModel.PlantKind;
import io.github.cumsauce.foley.api.SoundModel.SettlingStone;
import io.github.cumsauce.foley.api.SoundModel.Surf;
import io.github.cumsauce.foley.api.SoundModel.Voice;
import io.github.cumsauce.foley.api.SoundModel.Woodpecker;
import java.util.Locale;
import java.util.Map;
import net.minecraft.util.ExtraCodecs;

/**
 * {@link SoundModel} as data: a {@code "model"} field says which one, and the rest are its parameters:
 *
 * <pre>{@code
 * { "model": "drip_into_water", "radius_mm": 1.2 }
 * }</pre>
 *
 * The same codec reads what code registers and what a resource pack defines, so the two can never drift apart. Field
 * names follow {@code foley.h}'s vocabulary; the ranges are the records' own.
 */
public final class SoundCodecs {
	/** Every model, by the {@code "model"} name it is written as. */
	public static final Codec<SoundModel> CODEC = new Dispatch();

	/** A drop into water ({@code radius_mm} 0.1 to 10). */
	public static final MapCodec<DripIntoWater> DRIP_INTO_WATER = RecordCodecBuilder.mapCodec(instance -> instance.group(
			Codec.floatRange(0.1f, 10f).fieldOf("radius_mm").forGetter(DripIntoWater::radiusMm)).apply(instance,
			DripIntoWater::new));

	/** A drop on stone ({@code radius_mm} 0.1 to 10). */
	public static final MapCodec<DripOnStone> DRIP_ON_STONE = RecordCodecBuilder.mapCodec(instance -> instance.group(
			Codec.floatRange(0.1f, 10f).fieldOf("radius_mm").forGetter(DripOnStone::radiusMm)).apply(instance,
			DripOnStone::new));

	/** A drop into lava: a thud and a sizzle. */
	public static final MapCodec<LavaDrip> LAVA_DRIP = MapCodec.unit(new LavaDrip());

	/** Crickets ({@code celsius} −30 to 50). */
	public static final MapCodec<Crickets> CRICKETS = RecordCodecBuilder.mapCodec(instance -> instance.group(
			Codec.floatRange(-30f, 50f).fieldOf("celsius").forGetter(Crickets::celsius)).apply(instance, Crickets::new));

	/** A bird's song ({@code species} 0 to 1000). */
	public static final MapCodec<Birdsong> BIRDSONG = RecordCodecBuilder.mapCodec(instance -> instance.group(
			ExtraCodecs.intRange(0, 1000).fieldOf("species").forGetter(Birdsong::species)).apply(instance,
			Birdsong::new));

	/** A frog ({@code length_cm} 2 to 20). */
	public static final MapCodec<Frog> FROG = RecordCodecBuilder.mapCodec(instance -> instance.group(
			Codec.floatRange(2f, 20f).fieldOf("length_cm").forGetter(Frog::lengthCm)).apply(instance, Frog::new));

	/** A wave ({@code wave_height_m} 0.2 to 3). */
	public static final MapCodec<Surf> SURF = RecordCodecBuilder.mapCodec(instance -> instance.group(
			Codec.floatRange(0.2f, 3f).fieldOf("wave_height_m").forGetter(Surf::waveHeightM)).apply(instance,
			Surf::new));

	/** A woodpecker. */
	public static final MapCodec<Woodpecker> WOODPECKER = MapCodec.unit(new Woodpecker());

	/** An owl. */
	public static final MapCodec<Owl> OWL = MapCodec.unit(new Owl());

	/** Cicadas ({@code celsius} −30 to 50). */
	public static final MapCodec<Cicadas> CICADAS = RecordCodecBuilder.mapCodec(instance -> instance.group(
			Codec.floatRange(-30f, 50f).fieldOf("celsius").forGetter(Cicadas::celsius)).apply(instance, Cicadas::new));

	/** A wolf's howl. */
	public static final MapCodec<Howl> HOWL = MapCodec.unit(new Howl());

	/** Brushing through plants ({@code plant}: grass, leaves, dry). */
	public static final MapCodec<Brush> BRUSH = RecordCodecBuilder.mapCodec(instance -> instance.group(
			named("plant", PlantKind.values()).fieldOf("plant").forGetter(Brush::plant)).apply(instance, Brush::new));

	/** Lake ice singing. */
	public static final MapCodec<IceSinging> ICE_SINGING = MapCodec.unit(new IceSinging());

	/** Stones settling. */
	public static final MapCodec<SettlingStone> SETTLING_STONE = MapCodec.unit(new SettlingStone());

	/** A bird flushed. */
	public static final MapCodec<Flush> FLUSH = MapCodec.unit(new Flush());

	/** An insect's buzz ({@code insect}: bee, fly). */
	public static final MapCodec<Buzz> BUZZ = RecordCodecBuilder.mapCodec(instance -> instance.group(
			named("insect", InsectKind.values()).fieldOf("insect").forGetter(Buzz::insect)).apply(instance,
			Buzz::new));

	/** A bubble's ring ({@code radius_mm} 0.05 to 20). */
	public static final MapCodec<Bubble> BUBBLE = RecordCodecBuilder.mapCodec(instance -> instance.group(
			Codec.floatRange(0.05f, 20f).fieldOf("radius_mm").forGetter(Bubble::radiusMm)).apply(instance,
			Bubble::new));

	/** A burst of shaped noise ({@code centre_hz}, {@code q}, {@code seconds}, {@code gain}). */
	public static final MapCodec<NoiseBurst> NOISE_BURST = RecordCodecBuilder.mapCodec(instance -> instance.group(
			Codec.floatRange(20f, 20_000f).fieldOf("centre_hz").forGetter(NoiseBurst::centreHz),
			Codec.floatRange(0.5f, 20f).fieldOf("q").forGetter(NoiseBurst::q),
			Codec.floatRange(0.005f, 5f).fieldOf("seconds").forGetter(NoiseBurst::seconds),
			Codec.floatRange(0f, 1f).fieldOf("gain").forGetter(NoiseBurst::gain)).apply(instance, NoiseBurst::new));

	/** A gliding tone ({@code centre_hz}, {@code target_hz}, {@code seconds}, {@code gain}). */
	public static final MapCodec<Tone> TONE = RecordCodecBuilder.mapCodec(instance -> instance.group(
			Codec.floatRange(20f, 10_000f).fieldOf("centre_hz").forGetter(Tone::centreHz),
			Codec.floatRange(20f, 20_000f).fieldOf("target_hz").forGetter(Tone::targetHz),
			Codec.floatRange(0.01f, 5f).fieldOf("seconds").forGetter(Tone::seconds),
			Codec.floatRange(0f, 1f).fieldOf("gain").forGetter(Tone::gain)).apply(instance, Tone::new));

	/** A gust ({@code speed} m/s, {@code seconds}, {@code gain}). */
	public static final MapCodec<Gust> GUST = RecordCodecBuilder.mapCodec(instance -> instance.group(
			Codec.floatRange(0f, 60f).fieldOf("speed").forGetter(Gust::speed),
			Codec.floatRange(0.2f, 10f).fieldOf("seconds").forGetter(Gust::seconds),
			Codec.floatRange(0f, 1f).fieldOf("gain").forGetter(Gust::gain)).apply(instance, Gust::new));

	/** A body meeting a material: {@code kind}, {@code material}, {@code speed} (m/s, 0.1 to 30). */
	public static final MapCodec<Contact> CONTACT = RecordCodecBuilder.mapCodec(instance -> instance.group(
			named("kind", SoundModel.ContactKind.values()).fieldOf("kind").forGetter(Contact::kind),
			named("material", SoundModel.MaterialKind.values()).fieldOf("material").forGetter(Contact::material),
			Codec.floatRange(0.1f, 30f).fieldOf("speed").forGetter(Contact::speed)).apply(instance, Contact::new));

	/** A mob's call ({@code mob}, {@code call}). */
	public static final MapCodec<Voice> VOICE = RecordCodecBuilder.mapCodec(instance -> instance.group(
			Codec.STRING.fieldOf("mob").forGetter(Voice::mob),
			Codec.STRING.fieldOf("call").forGetter(Voice::call)).apply(instance, Voice::new));

	private static final String MODEL = "model";

	private static final Map<String, MapCodec<? extends SoundModel>> BY_KEY = Map.ofEntries(
			Map.entry("drip_into_water", DRIP_INTO_WATER),
			Map.entry("drip_on_stone", DRIP_ON_STONE),
			Map.entry("lava_drip", LAVA_DRIP),
			Map.entry("crickets", CRICKETS),
			Map.entry("birdsong", BIRDSONG),
			Map.entry("frog", FROG),
			Map.entry("surf", SURF),
			Map.entry("woodpecker", WOODPECKER),
			Map.entry("owl", OWL),
			Map.entry("cicadas", CICADAS),
			Map.entry("howl", HOWL),
			Map.entry("brush", BRUSH),
			Map.entry("ice_singing", ICE_SINGING),
			Map.entry("settling_stone", SETTLING_STONE),
			Map.entry("flush", FLUSH),
			Map.entry("buzz", BUZZ),
			Map.entry("bubble", BUBBLE),
			Map.entry("noise", NOISE_BURST),
			Map.entry("tone", TONE),
			Map.entry("gust", GUST),
			Map.entry("contact", CONTACT),
			Map.entry("voice", VOICE));

	private SoundCodecs() {}

	/** An enum written lowercase ({@code "leaves"}), with the values named in any error. Shared with the other codecs. */
	static <E extends Enum<E>> Codec<E> named(String what, E[] values) {
		return Codec.STRING.flatXmap(name -> {
			for (E value : values) {
				if (value.name().toLowerCase(Locale.ROOT).equals(name)) return DataResult.success(value);
			}
			return DataResult.error(() -> "unknown " + what + " \"" + name + "\"");
		}, value -> DataResult.success(value.name().toLowerCase(Locale.ROOT)));
	}

	/** The one name each model is written as. */
	private static String keyOf(SoundModel model) {
		if (model instanceof DripIntoWater) return "drip_into_water";
		if (model instanceof DripOnStone) return "drip_on_stone";
		if (model instanceof LavaDrip) return "lava_drip";
		if (model instanceof Crickets) return "crickets";
		if (model instanceof Birdsong) return "birdsong";
		if (model instanceof Frog) return "frog";
		if (model instanceof Surf) return "surf";
		if (model instanceof Woodpecker) return "woodpecker";
		if (model instanceof Owl) return "owl";
		if (model instanceof Cicadas) return "cicadas";
		if (model instanceof Howl) return "howl";
		if (model instanceof Brush) return "brush";
		if (model instanceof IceSinging) return "ice_singing";
		if (model instanceof SettlingStone) return "settling_stone";
		if (model instanceof Flush) return "flush";
		if (model instanceof Buzz) return "buzz";
		if (model instanceof Bubble) return "bubble";
		if (model instanceof NoiseBurst) return "noise";
		if (model instanceof Tone) return "tone";
		if (model instanceof Gust) return "gust";
		if (model instanceof Contact) return "contact";
		if (model instanceof Voice) return "voice";
		throw new IllegalArgumentException("unknown model " + model.getClass().getName());
	}

	/**
	 * Reads the {@code "model"} field to pick the model, then its fields from the same object; writes both together.
	 * Written by hand rather than through DFU's dispatch so the shape stays exactly {@code {"model": …, …}}.
	 */
	private static final class Dispatch implements Codec<SoundModel> {
		@Override
		public <T> DataResult<T> encode(SoundModel input, DynamicOps<T> ops, T prefix) {
			RecordBuilder<T> builder = ops.mapBuilder();
			builder.add(MODEL, ops.createString(keyOf(input)));
			codecOf(input).encode(input, ops, builder);
			return builder.build(prefix);
		}

		@Override
		public <T> DataResult<Pair<SoundModel, T>> decode(DynamicOps<T> ops, T input) {
			return decodeModel(ops, input).map(model -> Pair.of(model, input));
		}

		private <T> DataResult<SoundModel> decodeModel(DynamicOps<T> ops, T input) {
			return ops.getMap(input).flatMap(like -> resolve(ops, like));
		}

		/** The model a map names, by its {@code "model"} field. */
		private static <T> DataResult<SoundModel> resolve(DynamicOps<T> ops, MapLike<T> like) {
			T key = like.get(MODEL);
			if (key == null) return DataResult.error(() -> "no \"" + MODEL + "\" field");
			return ops.getStringValue(key).flatMap(name -> {
				MapCodec<? extends SoundModel> codec = BY_KEY.get(name);
				if (codec == null) return DataResult.error(() -> "unknown model \"" + name + "\"");
				return codec.decode(ops, like).map(model -> (SoundModel) model);
			});
		}

		@SuppressWarnings("unchecked")
		private static <M extends SoundModel> MapCodec<M> codecOf(SoundModel model) {
			return (MapCodec<M>) BY_KEY.get(keyOf(model));
		}
	}
}
