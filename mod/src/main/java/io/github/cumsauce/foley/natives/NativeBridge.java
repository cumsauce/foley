package io.github.cumsauce.foley.natives;

import static java.lang.foreign.ValueLayout.ADDRESS;
import static java.lang.foreign.ValueLayout.JAVA_BYTE;
import static java.lang.foreign.ValueLayout.JAVA_FLOAT;
import static java.lang.foreign.ValueLayout.JAVA_INT;
import static java.lang.foreign.ValueLayout.JAVA_LONG;

import java.lang.foreign.Arena;
import java.lang.foreign.FunctionDescriptor;
import java.lang.foreign.Linker;
import java.lang.foreign.MemorySegment;
import java.lang.foreign.SymbolLookup;
import java.lang.foreign.ValueLayout;
import java.lang.invoke.MethodHandle;
import java.nio.charset.StandardCharsets;

/**
 * Downcall handles for the C ABI in {@code crates/foley-ffi/include/foley.h}, with typed wrappers that turn error
 * codes into {@link FoleyException}. Audio is 48 kHz mono 16-bit PCM.
 */
public final class NativeBridge {
	/** Must equal {@code FOLEY_ABI_VERSION} in {@code foley.h}. */
	public static final int ABI_VERSION = 1;
	public static final int SAMPLE_RATE = 48_000;

	private static final ValueLayout.OfLong SIZE_T = JAVA_LONG;

	private final MethodHandle abiVersion;
	private final MethodHandle rainCreate;
	private final MethodHandle rainDestroy;
	private final MethodHandle rainRender;
	private final MethodHandle renderDrip;
	private final MethodHandle lastError;

	public NativeBridge(SymbolLookup library) {
		Linker linker = Linker.nativeLinker();
		Linker.Option critical = Linker.Option.critical(true);
		abiVersion = linker.downcallHandle(find(library, "foley_abi_version"), FunctionDescriptor.of(JAVA_INT));
		rainCreate = linker.downcallHandle(find(library, "foley_rain_create"),
				FunctionDescriptor.of(JAVA_INT, JAVA_LONG, JAVA_FLOAT, ADDRESS), critical);
		rainDestroy = linker.downcallHandle(find(library, "foley_rain_destroy"), FunctionDescriptor.ofVoid(ADDRESS));
		rainRender = linker.downcallHandle(find(library, "foley_rain_render"),
				FunctionDescriptor.of(JAVA_INT, ADDRESS, ADDRESS, SIZE_T));
		renderDrip = linker.downcallHandle(find(library, "foley_render_drip"),
				FunctionDescriptor.of(JAVA_INT, JAVA_FLOAT, JAVA_LONG, ADDRESS, SIZE_T, ADDRESS), critical);
		lastError = linker.downcallHandle(find(library, "foley_last_error"),
				FunctionDescriptor.of(SIZE_T, ADDRESS, SIZE_T));
	}

	public int abiVersion() {
		return (int) invoke(abiVersion);
	}

	/** Starts a bed of rain on water at {@code rate} drops a second. Free it with {@link #rainDestroy}. */
	public MemorySegment rainCreate(long seed, float rate) {
		long[] out = new long[1];
		check((int) invoke(rainCreate, seed, rate, MemorySegment.ofArray(out)));
		return MemorySegment.ofAddress(out[0]);
	}

	public void rainDestroy(MemorySegment rain) {
		invoke(rainDestroy, rain);
	}

	/** Renders the next samples of {@code rain} into {@code out}, which must be native memory, 2 bytes a sample. */
	public void rainRender(MemorySegment rain, MemorySegment out) {
		check((int) invoke(rainRender, rain, out, out.byteSize() / 2));
	}

	/** A water drip whose bubble is {@code radiusMm} millimetres, as 16-bit PCM. */
	public short[] renderDrip(float radiusMm, long seed) {
		long[] needed = new long[1];
		check((int) invoke(renderDrip, radiusMm, seed, MemorySegment.ofArray(new short[0]), 0L,
				MemorySegment.ofArray(needed)));
		short[] out = new short[(int) needed[0]];
		check((int) invoke(renderDrip, radiusMm, seed, MemorySegment.ofArray(out), (long) out.length,
				MemorySegment.ofArray(needed)));
		return out;
	}

	private void check(int code) {
		if (code == 0) return;
		try (Arena arena = Arena.ofConfined()) {
			long cap = 1024;
			MemorySegment buf = arena.allocate(cap);
			long length = Math.min((long) invoke(lastError, buf, cap), cap);
			throw new FoleyException(code, new String(buf.asSlice(0, length).toArray(JAVA_BYTE), StandardCharsets.UTF_8));
		}
	}

	private static Object invoke(MethodHandle handle, Object... args) {
		try {
			return handle.invokeWithArguments(args);
		} catch (Throwable t) {
			throw new IllegalStateException("bad downcall " + handle.type(), t);
		}
	}

	private static MemorySegment find(SymbolLookup library, String name) {
		return library.find(name).orElseThrow(() -> new UnsatisfiedLinkError("missing native symbol " + name));
	}
}
