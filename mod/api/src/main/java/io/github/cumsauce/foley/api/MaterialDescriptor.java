package io.github.cumsauce.foley.api;

import com.mojang.serialization.Codec;
import com.mojang.serialization.MapCodec;
import com.mojang.serialization.codecs.RecordCodecBuilder;
import io.github.cumsauce.foley.api.SoundModel.MaterialKind;

/**
 * What a mod's own block is made of, as Foley's modal synthesis takes it: the physics a struck plate follows, and the
 * built-in {@linkplain SoundModel.MaterialKind material} whose habits it keeps besides — how it grains, scuffs,
 * crunches and cracks. Give one to {@link FoleyMaterials#register}, map blocks onto it, and Foley's strikes, breaks
 * and footsteps of those blocks come out with its physics: glassy ones ring bright and shatter into shards, stiff
 * heavy ones clang.
 *
 * <p>Every value is checked where it is built, against what the synthesis accepts; Foley's own ten materials sit
 * inside these ranges.
 */
public record MaterialDescriptor(
		MaterialKind behavior,
		double youngPa,
		double density,
		double poisson,
		double loss,
		double thickness,
		int fragments,
		double restitution,
		double shard) {

	/** Young's modulus, Pa: 0.1 GPa (soft rubber) to 500 GPa (stiffest steels). */
	public MaterialDescriptor {
		if (behavior == null) throw new IllegalArgumentException("behavior must be a built-in material's");
		check("young_pa", youngPa, 1e8, 5e11);
		check("density", density, 1, 20_000);
		check("poisson", poisson, 0, 0.49);
		check("loss", loss, 0.0005, 2.0);
		check("thickness", thickness, 0.001, 1.0);
		check("fragments", fragments, 0, 200);
		check("restitution", restitution, 0, 0.95);
		if (shard != 0 && !(0.0005 <= shard && shard <= 0.2))
			throw new IllegalArgumentException("shard must be 0 (chunks) or 0.0005 to 0.2, not " + shard);
	}

	/** Glassy (a pane's constants, 40 shards): bright taps, tinkling breaks. */
	public static MaterialDescriptor glassy() {
		return new MaterialDescriptor(MaterialKind.GLASS, 70e9, 2500, 0.22, 0.004, 0.01, 40, 0.5, 0.004);
	}

	/** Metal (steel, 5 cm): a clang that rings for seconds, a dull blow when it gives. */
	public static MaterialDescriptor metallic() {
		return new MaterialDescriptor(MaterialKind.METAL, 200e9, 7850, 0.3, 0.004, 0.05, 0, 0.6, 0);
	}

	/** Stony (granite, a 25 cm slab): clicks, a crumble of fragments. */
	public static MaterialDescriptor stony() {
		return new MaterialDescriptor(MaterialKind.STONE, 50e9, 2700, 0.25, 0.04, 0.25, 16, 0.35, 0);
	}

	/** Wooden (2.5 cm boards): a knock, fibres crackling as it gives. */
	public static MaterialDescriptor wooden() {
		return new MaterialDescriptor(MaterialKind.WOOD, 10e9, 500, 0.3, 0.025, 0.025, 6, 0.4, 0.01);
	}

	/** Soft (wool, 30 cm): a dull thud, hardly a ring at all. */
	public static MaterialDescriptor soft() {
		return new MaterialDescriptor(MaterialKind.SOFT, 1e6, 300, 0.3, 0.6, 0.3, 0, 0.1, 0);
	}

	/** The physics as the synthesis takes them, one float per value. */
	public float[] toFloats() {
		return new float[] {behavior.ordinal(), (float) youngPa, (float) density, (float) poisson, (float) loss,
				(float) thickness, fragments, (float) restitution, shard == 0 ? 0f : (float) shard};
	}

	/** Reads one from a material definition's JSON (see {@code docs/INTEGRATING.md}). */
	public static final MapCodec<MaterialDescriptor> CODEC = RecordCodecBuilder.mapCodec(instance -> instance.group(
			SoundCodecs.named("behavior", MaterialKind.values()).fieldOf("behavior")
					.forGetter(MaterialDescriptor::behavior),
			Codec.doubleRange(0.1, 500).fieldOf("young_gpa").forGetter(d -> d.youngPa / 1e9),
			Codec.doubleRange(1, 20_000).fieldOf("density").forGetter(MaterialDescriptor::density),
			Codec.doubleRange(0, 0.49).fieldOf("poisson").forGetter(MaterialDescriptor::poisson),
			Codec.doubleRange(0.0005, 2.0).fieldOf("loss").forGetter(MaterialDescriptor::loss),
			Codec.doubleRange(0.001, 1.0).fieldOf("thickness").forGetter(MaterialDescriptor::thickness),
			Codec.intRange(0, 200).optionalFieldOf("fragments", 0).forGetter(MaterialDescriptor::fragments),
			Codec.doubleRange(0, 0.95).optionalFieldOf("restitution", 0.1).forGetter(MaterialDescriptor::restitution),
			Codec.doubleRange(0, 0.2).optionalFieldOf("shard", 0.0).forGetter(d -> d.shard == 0 ? 0.0 : d.shard))
			.apply(instance, (behavior, youngGpa, density, poisson, loss, thickness, fragments, restitution, shard) ->
					new MaterialDescriptor(behavior, youngGpa * 1e9, density, poisson, loss, thickness, fragments,
							restitution, shard)));

	private static void check(String name, double value, double min, double max) {
		if (!(min <= value && value <= max))
			throw new IllegalArgumentException(name + " must be " + min + " to " + max + ", not " + value);
	}

	private static void check(String name, int value, int min, int max) {
		if (!(min <= value && value <= max))
			throw new IllegalArgumentException(name + " must be " + min + " to " + max + ", not " + value);
	}
}
