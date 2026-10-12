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
	public static final int ABI_VERSION = 5;
	/** Surfaces rain lands on, in the order its scene takes them. */
	public static final int SURFACES = 7;
	public static final int SAMPLE_RATE = 48_000;
	/** Bed kinds ({@code FOLEY_BED_*}). */
	public static final int BED_RAIN = 0;
	public static final int BED_WIND = 1;
	public static final int BED_WATER = 2;
	public static final int BED_FIRE = 3;
	public static final int BED_LAVA = 4;
	public static final int BED_THUNDER = 5;
	/** One-shot kinds ({@code FOLEY_SHOT_*}). */
	public static final int SHOT_DRIP_INTO_WATER = 0;
	public static final int SHOT_DRIP_ON_STONE = 1;
	public static final int SHOT_LAVA_DRIP = 2;

	private static final ValueLayout.OfLong SIZE_T = JAVA_LONG;

	private final MethodHandle abiVersion;
	private final MethodHandle bedCreate;
	private final MethodHandle bedSetScene;
	private final MethodHandle bedRender;
	private final MethodHandle bedDestroy;
	private final MethodHandle bedStrike;
	private final MethodHandle renderOneShot;
	private final MethodHandle lastError;

	public NativeBridge(SymbolLookup library) {
		Linker linker = Linker.nativeLinker();
		Linker.Option critical = Linker.Option.critical(true);
		abiVersion = linker.downcallHandle(find(library, "foley_abi_version"), FunctionDescriptor.of(JAVA_INT));
		bedCreate = linker.downcallHandle(find(library, "foley_bed_create"),
				FunctionDescriptor.of(JAVA_INT, JAVA_INT, JAVA_LONG, ADDRESS), critical);
		bedSetScene = linker.downcallHandle(find(library, "foley_bed_set_scene"),
				FunctionDescriptor.of(JAVA_INT, ADDRESS, ADDRESS, SIZE_T), critical);
		bedRender = linker.downcallHandle(find(library, "foley_bed_render"),
				FunctionDescriptor.of(JAVA_INT, ADDRESS, ADDRESS, SIZE_T));
		bedDestroy = linker.downcallHandle(find(library, "foley_bed_destroy"), FunctionDescriptor.ofVoid(ADDRESS));
		bedStrike = linker.downcallHandle(find(library, "foley_bed_strike"),
				FunctionDescriptor.of(JAVA_INT, ADDRESS, JAVA_FLOAT, JAVA_FLOAT, JAVA_FLOAT, JAVA_LONG), critical);
		renderOneShot = linker.downcallHandle(find(library, "foley_render_one_shot"),
				FunctionDescriptor.of(JAVA_INT, JAVA_INT, JAVA_FLOAT, JAVA_LONG, ADDRESS, SIZE_T, ADDRESS), critical);
		lastError = linker.downcallHandle(find(library, "foley_last_error"),
				FunctionDescriptor.of(SIZE_T, ADDRESS, SIZE_T));
	}

	public int abiVersion() {
		return (int) invoke(abiVersion);
	}

	/** Starts a bed of {@code kind} ({@code BED_*}), silent until {@link #bedSetScene}. Free it with {@link #bedDestroy}. */
	public MemorySegment bedCreate(int kind, long seed) {
		long[] out = new long[1];
		check((int) invoke(bedCreate, kind, seed, MemorySegment.ofArray(out)));
		return MemorySegment.ofAddress(out[0]);
	}

	/** What the world round a bed is like: the floats its kind takes, in order (see {@code foley.h}). */
	public void bedSetScene(MemorySegment bed, float[] params) {
		check((int) invoke(bedSetScene, bed, MemorySegment.ofArray(params), (long) params.length));
	}

	/** Renders the next samples of {@code bed} into {@code out}, which must be native memory, 2 bytes a sample. */
	public void bedRender(MemorySegment bed, MemorySegment out) {
		check((int) invoke(bedRender, bed, out, out.byteSize() / 2));
	}

	public void bedDestroy(MemorySegment bed) {
		invoke(bedDestroy, bed);
	}

	/** A flash striking the ground at ({@code x}, {@code y}, {@code z}) blocks from the listener, for a thunder bed. */
	public void bedStrike(MemorySegment bed, float x, float y, float z, long seed) {
		check((int) invoke(bedStrike, bed, x, y, z, seed));
	}

	/** A one-shot of {@code kind} ({@code SHOT_*}), as 16-bit PCM; {@code sizeMm} is a drip's radius. */
	public short[] renderOneShot(int kind, float sizeMm, long seed) {
		long[] needed = new long[1];
		check((int) invoke(renderOneShot, kind, sizeMm, seed, MemorySegment.ofArray(new short[0]), 0L,
				MemorySegment.ofArray(needed)));
		short[] out = new short[(int) needed[0]];
		check((int) invoke(renderOneShot, kind, sizeMm, seed, MemorySegment.ofArray(out), (long) out.length,
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
