package io.github.cumsauce.foley.api;

import com.mojang.serialization.Codec;
import com.mojang.serialization.MapCodec;
import com.mojang.serialization.codecs.RecordCodecBuilder;

/**
 * A creature's voice, as Foley's synthesis takes it: its vocal tract's length and usual pitch — which together read
 * as the body's size — and how far the creature goes into roughness (the nonlinear phenomena of alarm: chaos,
 * subharmonics, creak) and breath. Foley makes the five calls any creature needs from it: ambient, hurt, death,
 * growl, breath.
 *
 * <p>A human-like creature is a 17 cm tract at about 120 Hz; a bird, 5 cm at a kilohertz; a giant, 60 cm at 70. Every
 * value is checked where it is built.
 */
public record VoiceDescriptor(double tractLengthM, double pitchHz, double rough, double breathy) {
	public VoiceDescriptor {
		check("tract_length_m", tractLengthM, 0.03, 1.0);
		check("pitch_hz", pitchHz, 30, 4000);
		check("rough", rough, 0, 1);
		check("breathy", breathy, 0, 1);
	}

	/** Reads one from a voice definition's JSON (see {@code docs/INTEGRATING.md}). */
	public static final MapCodec<VoiceDescriptor> CODEC = RecordCodecBuilder.mapCodec(instance -> instance.group(
			Codec.doubleRange(0.03, 1.0).fieldOf("tract_length_m").forGetter(VoiceDescriptor::tractLengthM),
			Codec.doubleRange(30, 4000).fieldOf("pitch_hz").forGetter(VoiceDescriptor::pitchHz),
			Codec.doubleRange(0, 1).optionalFieldOf("rough", 0.5).forGetter(VoiceDescriptor::rough),
			Codec.doubleRange(0, 1).optionalFieldOf("breathy", 0.5).forGetter(VoiceDescriptor::breathy))
			.apply(instance, VoiceDescriptor::new));

	/** The physics as the synthesis takes them, one float per value. */
	public float[] toFloats() {
		return new float[] {(float) tractLengthM, (float) pitchHz, (float) rough, (float) breathy};
	}

	private static void check(String name, double value, double min, double max) {
		if (!(min <= value && value <= max))
			throw new IllegalArgumentException(name + " must be " + min + " to " + max + ", not " + value);
	}
}
