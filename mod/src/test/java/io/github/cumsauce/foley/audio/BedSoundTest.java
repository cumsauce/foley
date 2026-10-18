package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotSame;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.github.cumsauce.foley.natives.NativeBridge;
import java.lang.foreign.MemorySegment;
import java.util.concurrent.atomic.AtomicInteger;
import net.minecraft.sounds.SoundSource;
import org.junit.jupiter.api.Test;

/**
 * The native bed's lifetime across a reopen: the game may ask a bed for a fresh stream (a resource
 * reload, the source asking again), and the previous native bed must be destroyed then — its
 * {@code close()} is what calls {@code bedDestroy} — rather than leaked for the rest of the session.
 */
class BedSoundTest {
	/** Counts the native calls a bed makes, with no library behind them. */
	private static final class CountingBridge implements BedSound.BedApi {
		final AtomicInteger created = new AtomicInteger();
		final AtomicInteger destroyed = new AtomicInteger();

		@Override
		public MemorySegment bedCreate(int kind, long seed) {
			created.incrementAndGet();
			return MemorySegment.ofArray(new byte[1]);
		}

		@Override
		public void bedSetScene(MemorySegment bed, float[] params) {}

		@Override
		public void bedRender(MemorySegment bed, MemorySegment out) {}

		@Override
		public void bedStrike(MemorySegment bed, float x, float y, float z, long seed) {}

		@Override
		public void bedDestroy(MemorySegment bed) {
			destroyed.incrementAndGet();
		}
	}

	@Test
	void reopening_a_bed_destroys_the_old_native_bed() throws Exception {
		CountingBridge bridge = new CountingBridge();
		BedSound<UnderwaterScene> sound = new BedSound<>("underwater", SoundSource.AMBIENT, bridge,
				NativeBridge.BED_UNDERWATER, UnderwaterScene.DRY);
		var first = sound.getAudioStream(null, null, true).join();
		assertEquals(1, bridge.created.get(), "first open must create a bed");
		var second = sound.getAudioStream(null, null, true).join();
		assertEquals(2, bridge.created.get(), "reopen must create a new bed");
		assertNotSame(first, second, "reopen must hand back a new stream");
		assertEquals(1, bridge.destroyed.get(), "the old native bed leaked: reopening did not destroy it");
		// The game may close the retired stream again later; that must not double-destroy.
		first.close();
		assertEquals(1, bridge.destroyed.get(), "closing the retired stream double-destroyed the bed");
		second.close();
		assertEquals(2, bridge.destroyed.get(), "closing the live stream must destroy its bed exactly once");
	}

	/**
	 * A bed starts silent and fades up over seconds, so a bed (re)starting — a scene's bed beginning, a slot
	 * flapping, a biome's four quarters re-entering — is a fade, not a pop. Fails before the fade: the weight
	 * used to start at 1.
	 */
	@Test
	void a_bed_fades_up_rather_than_starting_at_full_weight() {
		CountingBridge bridge = new CountingBridge();
		BedSound<UnderwaterScene> sound = new BedSound<>("underwater", SoundSource.AMBIENT, bridge,
				NativeBridge.BED_UNDERWATER, UnderwaterScene.DRY);
		assertEquals(0f, sound.heardWeight(), 1e-6, "a new bed must start silent");
		for (int i = 0; i < 20; i++) sound.tick();
		assertTrue(sound.heardWeight() > 0.5f, "twenty ticks in, the bed must be well on its way up");
		assertTrue(sound.heardWeight() < 1f, "and still gliding there, not stepped to full weight");
	}

	/**
	 * Fading out lets the game retire the bed only once its weight has glided away: the tail rings out over
	 * about 3 s instead of SoundManager.stop cutting the stream dead. (fadeOut is new; before it there was no
	 * graceful path at all.)
	 */
	@Test
	void fading_out_retires_the_bed_after_the_fade_not_at_once() {
		CountingBridge bridge = new CountingBridge();
		BedSound<UnderwaterScene> sound = new BedSound<>("underwater", SoundSource.AMBIENT, bridge,
				NativeBridge.BED_UNDERWATER, UnderwaterScene.DRY);
		for (int i = 0; i < 20; i++) sound.tick();
		sound.fadeOut();
		assertFalse(sound.isStopped(), "a fade must not retire the bed at once");
		for (int i = 0; i < 40; i++) sound.tick();
		assertFalse(sound.isStopped(), "nor halfway through it");
		for (int i = 0; i < 40; i++) sound.tick();
		assertTrue(sound.isStopped(), "after the fade the game must retire the bed");
	}
}
