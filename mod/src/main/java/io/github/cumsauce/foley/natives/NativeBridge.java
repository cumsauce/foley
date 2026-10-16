package io.github.cumsauce.foley.natives;

import static java.lang.foreign.ValueLayout.ADDRESS;
import static java.lang.foreign.ValueLayout.JAVA_BYTE;
import static java.lang.foreign.ValueLayout.JAVA_FLOAT;
import static java.lang.foreign.ValueLayout.JAVA_INT;
import static java.lang.foreign.ValueLayout.JAVA_LONG;
import static java.lang.foreign.ValueLayout.JAVA_SHORT;

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
	public static final int ABI_VERSION = 21;
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
	public static final int SHOT_SCULK_RING = 16;
	/** Contacts ({@code FOLEY_CONTACT_*}) with a material ({@code Materials}). */
	public static final int CONTACT_TAP = 0;
	public static final int CONTACT_PLACE = 1;
	public static final int CONTACT_LAND = 2;
	public static final int CONTACT_BREAK = 3;
	public static final int CONTACT_STEP = 4;

	/** Primitive kinds ({@code FOLEY_PRIMITIVE_*}), the floats each takes in {@code foley.h}. */
	public static final int PRIMITIVE_BUBBLE = 0;
	public static final int PRIMITIVE_NOISE = 1;
	public static final int PRIMITIVE_TONE = 2;
	public static final int PRIMITIVE_GUST = 3;

	private static final ValueLayout.OfLong SIZE_T = JAVA_LONG;

	private final MethodHandle abiVersion;
	private final MethodHandle bedCreate;
	private final MethodHandle bedSetScene;
	private final MethodHandle bedRender;
	private final MethodHandle bedDestroy;
	private final MethodHandle bedStrike;
	private final MethodHandle renderOneShot;
	private final MethodHandle renderContact;
	private final MethodHandle renderContactCustom;
	private final MethodHandle renderPrimitive;
	private final MethodHandle renderCreature;
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
				FunctionDescriptor.of(JAVA_INT, JAVA_INT, JAVA_FLOAT, JAVA_LONG, ADDRESS, SIZE_T, ADDRESS));
		renderContact = linker.downcallHandle(find(library, "foley_render_contact"),
				FunctionDescriptor.of(JAVA_INT, JAVA_INT, JAVA_INT, JAVA_FLOAT, JAVA_LONG, ADDRESS, SIZE_T, ADDRESS));
		renderContactCustom = linker.downcallHandle(find(library, "foley_render_contact_custom"),
				FunctionDescriptor.of(JAVA_INT, ADDRESS, SIZE_T, JAVA_INT, JAVA_FLOAT, JAVA_LONG, ADDRESS, SIZE_T,
						ADDRESS));
		renderPrimitive = linker.downcallHandle(find(library, "foley_render_primitive"),
				FunctionDescriptor.of(JAVA_INT, JAVA_INT, ADDRESS, SIZE_T, JAVA_LONG, ADDRESS, SIZE_T, ADDRESS));
		renderCreature = linker.downcallHandle(find(library, "foley_render_creature"),
				FunctionDescriptor.of(JAVA_INT, ADDRESS, SIZE_T, JAVA_INT, JAVA_LONG, ADDRESS, SIZE_T, ADDRESS));
		renderVoice = linker.downcallHandle(find(library, "foley_render_voice"),
				FunctionDescriptor.of(JAVA_INT, JAVA_INT, JAVA_INT, JAVA_LONG, ADDRESS, SIZE_T, ADDRESS));
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
		int code;
		try {
			// Exact, so the stream thread's call boxes and allocates nothing.
			code = (int) bedRender.invokeExact(bed, out, out.byteSize() / 2);
		} catch (Throwable t) {
			throw new IllegalStateException("bad downcall", t);
		}
		check(code);
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

	/**
	 * A {@code contact} ({@code CONTACT_*}) on a custom material's physics: {@code props} is the descriptor's floats
	 * (behaviour, E, ρ, ν, η, plate, fragments, restitution, shard), as 16-bit PCM.
	 */
	public short[] renderContactCustom(float[] props, int contact, float speed, long seed) {
		// The props are handed over in native memory: only critical downcalls (bedSetScene's) take heap segments.
		try (Arena arena = Arena.ofConfined()) {
			MemorySegment nativeProps = arena.allocate(JAVA_FLOAT, props.length);
			MemorySegment.copy(props, 0, nativeProps, JAVA_FLOAT, 0, props.length);
			return render((out, needed) -> invoke(renderContactCustom, nativeProps, (long) props.length, contact,
					speed, seed, out, (long) (out.byteSize() / 2), needed));
		}
	}

	/**
	 * A primitive ({@code PRIMITIVE_*}) built from {@code params} (its floats, as {@code foley.h} orders them), as
	 * 16-bit PCM.
	 */
	public short[] renderPrimitive(int kind, float[] params, long seed) {
		// The params go in native memory: only critical downcalls (bedSetScene's) take heap segments.
		try (Arena arena = Arena.ofConfined()) {
			MemorySegment nativeParams = arena.allocate(JAVA_FLOAT, params.length);
			MemorySegment.copy(params, 0, nativeParams, JAVA_FLOAT, 0, params.length);
			return render((out, needed) -> invoke(renderPrimitive, kind, nativeParams, (long) params.length, seed,
					out, (long) (out.byteSize() / 2), needed));
		}
	}

	/** A creature call ({@code CREATURE_*}) of a custom creature from its {@code spec} floats, as 16-bit PCM. */
	public short[] renderCreature(float[] spec, int call, long seed) {
		// The spec goes in native memory: only critical downcalls (bedSetScene's) take heap segments.
		try (Arena arena = Arena.ofConfined()) {
			MemorySegment nativeSpec = arena.allocate(JAVA_FLOAT, spec.length);
			MemorySegment.copy(spec, 0, nativeSpec, JAVA_FLOAT, 0, spec.length);
			return render((out, needed) -> invoke(renderCreature, nativeSpec, (long) spec.length, call, seed, out,
					(long) (out.byteSize() / 2), needed));
		}
	}

	/** Creature calls ({@code FOLEY_CREATURE_*}). */
	public static final int CREATURE_AMBIENT = 0;
	public static final int CREATURE_HURT = 1;
	public static final int CREATURE_DEATH = 2;
	public static final int CREATURE_GROWL = 3;
	public static final int CREATURE_BREATH = 4;

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

	/**
	 * Renders once into a buffer big enough for nearly every sound, again only if it wasn't. Into native memory, not a
	 * Java array: a render can take tens of milliseconds, too long to hold off the garbage collector as a critical
	 * call on the heap would.
	 */
	private short[] render(Render render) {
		try (Arena arena = Arena.ofConfined()) {
			MemorySegment needed = arena.allocate(SIZE_T);
			MemorySegment out = arena.allocate(JAVA_SHORT, USUAL);
			check((int) render.call(out, needed));
			long length = needed.get(SIZE_T, 0);
			if (length > USUAL) {
				out = arena.allocate(JAVA_SHORT, length);
				check((int) render.call(out, needed));
				length = needed.get(SIZE_T, 0);
			}
			return out.asSlice(0, length * 2).toArray(JAVA_SHORT);
		}
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
