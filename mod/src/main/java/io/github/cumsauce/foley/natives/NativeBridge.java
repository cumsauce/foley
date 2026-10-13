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
	public static final int ABI_VERSION = 14;
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
	public static final int SHOT_CRICKETS = 3;
	public static final int SHOT_BIRDSONG = 4;
	public static final int SHOT_FROG = 5;
	public static final int SHOT_SURF = 6;
	public static final int SHOT_WOODPECKER = 7;
	public static final int SHOT_OWL = 8;
	public static final int SHOT_CICADAS = 9;
	public static final int SHOT_HOWL = 10;
	public static final int SHOT_BRUSH = 11;
	public static final int SHOT_ICE_SINGING = 12;
	public static final int SHOT_SETTLING_STONE = 13;
	public static final int SHOT_FLUSH = 14;
	public static final int SHOT_BUZZ = 15;
	/** Contacts ({@code FOLEY_CONTACT_*}) with a material ({@code Materials}). */
	public static final int CONTACT_TAP = 0;
	public static final int CONTACT_PLACE = 1;
	public static final int CONTACT_LAND = 2;
	public static final int CONTACT_BREAK = 3;
	public static final int CONTACT_STEP = 4;

	private static final ValueLayout.OfLong SIZE_T = JAVA_LONG;

	private final MethodHandle abiVersion;
	private final MethodHandle bedCreate;
	private final MethodHandle bedSetScene;
	private final MethodHandle bedRender;
	private final MethodHandle bedDestroy;
	private final MethodHandle bedStrike;
	private final MethodHandle renderOneShot;
	private final MethodHandle renderContact;
	private final MethodHandle renderVoice;
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
		renderContact = linker.downcallHandle(find(library, "foley_render_contact"),
				FunctionDescriptor.of(JAVA_INT, JAVA_INT, JAVA_INT, JAVA_FLOAT, JAVA_LONG, ADDRESS, SIZE_T, ADDRESS),
				critical);
		renderVoice = linker.downcallHandle(find(library, "foley_render_voice"),
				FunctionDescriptor.of(JAVA_INT, JAVA_INT, JAVA_INT, JAVA_LONG, ADDRESS, SIZE_T, ADDRESS), critical);
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

	/**
	 * A one-shot of {@code kind} ({@code SHOT_*}), as 16-bit PCM. {@code size} is a drip's radius (mm), the air's
	 * temperature for crickets (°C), a bird's species, or a frog's length (cm).
	 */
	public short[] renderOneShot(int kind, float size, long seed) {
		return render((out, needed) -> invoke(renderOneShot, kind, size, seed, out, (long) (out.byteSize() / 2), needed));
	}

	/**
	 * A {@code contact} ({@code CONTACT_*}) with a block of {@code material} ({@code Materials}) at {@code speed} m/s,
	 * as 16-bit PCM.
	 */
	public short[] renderContact(int contact, int material, float speed, long seed) {
		return render((out, needed) -> invoke(renderContact, contact, material, speed, seed, out, (long) (out.byteSize() / 2), needed));
	}

	/** A {@code call} ({@code Voices.CALLS}) of {@code mob} ({@code Voices.MOBS}), as 16-bit PCM. */
	public short[] renderVoice(int mob, int call, long seed) {
		return render((out, needed) -> invoke(renderVoice, mob, call, seed, out, (long) (out.byteSize() / 2), needed));
	}

	/** One native render call: into {@code out} (16-bit samples), its length to {@code needed}; the status. */
	private interface Render {
		Object call(MemorySegment out, MemorySegment needed);
	}

	/** Most sounds fit this many samples (8 s), so they're made once; a longer one is made again to fit. */
	private static final int USUAL = 8 * 48_000;

	/** Renders once into a buffer big enough for nearly every sound, again only if it wasn't. */
	private short[] render(Render render) {
		long[] needed = new long[1];
		short[] out = new short[USUAL];
		check((int) render.call(MemorySegment.ofArray(out), MemorySegment.ofArray(needed)));
		if (needed[0] > out.length) {
			out = new short[Math.toIntExact(needed[0])];
			check((int) render.call(MemorySegment.ofArray(out), MemorySegment.ofArray(needed)));
		}
		return java.util.Arrays.copyOf(out, (int) needed[0]);
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
