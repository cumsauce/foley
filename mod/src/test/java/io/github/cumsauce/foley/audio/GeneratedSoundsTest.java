package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;

import io.github.cumsauce.foley.natives.NativeBridge;
import java.lang.reflect.Method;
import org.junit.jupiter.api.Test;

/**
 * Generated one-shots: a bad variant name (from sounds.json or a pack) is "no sound" — the {@code otherwise} path —
 * never an exception, and never an out-of-range index handed to native.
 */
class GeneratedSoundsTest {
	/** {@code GeneratedSounds.life} by reflection: it is private, and its behaviour is what sounds.json depends on. */
	private static Object life(NativeBridge bridge, String model, int variant, short[] otherwise) throws Exception {
		Method life = GeneratedSounds.class.getDeclaredMethod("life", NativeBridge.class, String.class, int.class,
				short[].class);
		life.setAccessible(true);
		return life.invoke(null, bridge, model, variant, otherwise);
	}

	@Test
	void outOfRangeVariantReturnsOtherwise() throws Exception {
		short[] otherwise = {1, 2, 3};
		// life() re-parses its band index n from the model name ("crickets_5"): a table-indexed
		// model bounds-checks n before indexing. The bridge is never reached on the out-of-range
		// path, so null is safe here.
		assertSame(otherwise, life(null, "crickets_" + Life.CRICKET_CELSIUS.length, 0, otherwise));
		assertSame(otherwise, life(null, "crickets_-1", 0, otherwise));
		assertSame(otherwise, life(null, "frog_" + Life.FROG_CM.length, 0, otherwise));
		assertSame(otherwise, life(null, "cicadas_999", 0, otherwise));
		// The surf one-shot arm is gone (M17): "surf_0" is just an unknown model again.
		assertSame(otherwise, life(null, "surf_0", 0, otherwise));
		// An unknown model is still otherwise.
		assertSame(otherwise, life(null, "warbler_0", 0, otherwise));
	}

	@Test
	void emptyPcmIsNotPlayable() {
		// Both render paths wrap through AddonSounds.toBuffer: null and zero-length PCM are "no sound".
		assertNull(AddonSounds.toBuffer(BedSound.FORMAT, new short[0]));
		assertNull(AddonSounds.toBuffer(BedSound.FORMAT, null));
	}
}
