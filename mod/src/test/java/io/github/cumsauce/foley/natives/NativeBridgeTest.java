package io.github.cumsauce.foley.natives;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Test;

/**
 * The one-shot render path slices a native-written buffer by the length the native side reports back. These tests
 * pin the slice bound; they need no JNI, only the pure {@code sliceLength} seam.
 */
class NativeBridgeTest {
	@Test
	void a_retry_reporting_more_than_capacity_throws() {
		// The retry asked the native side for exactly this many samples; a bigger report on the way back would
		// slice into uninitialized native memory and play it as PCM, so the bridge must refuse.
		int usual = 8 * 48_000;
		assertThrows(FoleyException.class, () -> NativeBridge.sliceLength(usual + 1, usual));
		assertEquals(usual, NativeBridge.sliceLength(usual, usual));
		assertEquals(0, NativeBridge.sliceLength(0, usual));
		assertEquals(1, NativeBridge.sliceLength(1, 1));
		// A size_t past Long.MAX_VALUE reads back negative: refused too, never sliced.
		assertThrows(FoleyException.class, () -> NativeBridge.sliceLength(-1, usual));
	}

	@Test
	void a_report_within_capacity_is_kept() {
		int usual = 8 * 48_000;
		assertEquals(480, NativeBridge.sliceLength(480, usual));
		assertEquals(usual, NativeBridge.sliceLength(usual, usual));
	}
}
