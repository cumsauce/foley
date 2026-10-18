package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.mixin.AbstractSoundInstanceAccessor;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.core.BlockPos;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.NoteBlock;
import net.minecraft.world.level.block.state.BlockState;
import java.util.List;
import java.util.concurrent.atomic.AtomicInteger;
import org.jspecify.annotations.Nullable;

/**
 * The sixteen vanilla note block instruments, in the order the native synth's table gives them, and the names of the
 * buffers Foley renders for their strikes ({@code foley:generated/note_<instrument>_<note>_<variant>}).
 */
public final class NoteBlocks {
	/** The instruments ({@code harp} … {@code pling}), in the order {@code foley_synth}'s table maps them. */
	public static final List<String> INSTRUMENTS = List.of(
			"harp", "basedrum", "snare", "hat", "bass", "flute", "bell", "guitar", "chime", "xylophone",
			"iron_xylophone", "cow_bell", "didgeridoo", "bit", "banjo", "pling");

	/** Foley's note block event ({@code foley:note}), for naming its buffers (it has no sounds.json entry). */
	public static final SoundEvent EVENT = Sounds.event("note");

	/** How many timbres each note is struck with; the instance's hash picks one of them. */
	static final int VARIANTS = 8;

	/**
	 * How many strikes Foley has taken for its own instrument. Unlike {@code FoleyClient.replaced()} this counts the
	 * strikes themselves rather than the sounds skipped, so a caller can tell a note Foley played from a note that fell
	 * through to the game. For the client gametest.
	 */
	private static final AtomicInteger PLAYS = new AtomicInteger();

	private NoteBlocks() {}

	/** How many note block strikes Foley has played. For the client gametest. */
	public static int plays() {
		return PLAYS.get();
	}

	/** The instrument a vanilla note block sound id names ({@code block.note_block.<instrument>}), or −1. */
	public static int instrument(Identifier id) {
		String path = id.getPath();
		String prefix = "block.note_block.";
		if (!path.startsWith(prefix)) return -1;
		return INSTRUMENTS.indexOf(path.substring(prefix.length()));
	}

	/** The generated buffer for a strike ({@code foley:generated/note_<instrument>_<note>_<variant>}). */
	public static Identifier bufferName(int instrument, int note, int variant) {
		return Identifier.fromNamespaceAndPath("foley",
				GeneratedSounds.PREFIX + "note_" + instrument + "_" + note + "_" + variant);
	}

	/** The note (0 to 24) the block at that spot is tuned to, or −1 if it is not a note block (or unreadable). */
	public static int noteAt(@Nullable Minecraft minecraft, double x, double y, double z) {
		if (minecraft == null || minecraft.level == null) return -1;
		BlockState state = minecraft.level.getBlockState(BlockPos.containing(x, y, z));
		return state.is(Blocks.NOTE_BLOCK) ? state.getValue(NoteBlock.NOTE) : -1;
	}

	/**
	 * The note a played note block sound strikes: the block's tuning where it can be read, else the one heard from the
	 * instance's pitch (vanilla plays {@code 2 ^ ((note − 12) / 12)}).
	 */
	public static int noteOf(SoundInstance sound, @Nullable Minecraft minecraft) {
		int note = noteAt(minecraft, sound.getX(), sound.getY(), sound.getZ());
		if (note >= 0) return note;
		// Vanilla's getter reads the resolved sound, which a strike heard before playback (or outside the game) lacks.
		float pitch = sound instanceof AbstractSoundInstanceAccessor own ? own.foley$pitch() : 1f;
		int heard = pitch > 0 ? Math.round(12f + 12f * (float) (Math.log(pitch) / Math.log(2))) : 12;
		return Math.max(0, Math.min(24, heard));
	}

	/**
	 * Whether Foley takes this strike from the game: the id names an instrument Foley knows and the {@code noteBlocks}
	 * setting is on, so its own buffer for the note plays at the sound's place and the game's sound is skipped. An
	 * unknown id, or the setting off, leaves the game's sound alone.
	 *
	 * <p>The variant comes from the instance's own hash: the sound's seed is not readable in these mappings (the MC
	 * 26.3 {@link SoundInstance} exposes no seed, verified with javap), and the game builds a fresh instance per
	 * strike, so the identity hash picks a different one of the {@value #VARIANTS} timbres each time and successive
	 * hits differ.
	 */
	public static boolean replace(SoundInstance sound, @Nullable Minecraft minecraft) {
		int instrument = instrument(sound.getIdentifier());
		if (instrument < 0) return false;
		// Bypass always lets the game's sound through, without consulting an override.
		if (FoleyClient.bypassed()) return false;
		// An override, if one matches, owns the strike entirely — like every other takeover family.
		// It is consulted before the toggle: what a mod claims, it keeps.
		DoorHandling.Action action = DoorHandling.dispatch(sound.getIdentifier(), sound.getX(), sound.getY(),
				sound.getZ());
		if (action == DoorHandling.Action.OVERRIDE_HANDLED) return true;
		if (action == DoorHandling.Action.OVERRIDE_VANILLA) return false;
		if (!FoleyClient.wants(FoleyConfig::noteBlocks)) return false;
		int note = noteOf(sound, minecraft);
		int variant = Math.floorMod(sound.hashCode(), VARIANTS);
		if (minecraft != null) {
			minecraft.getSoundManager().play(AddonSounds.instance(EVENT.location(), bufferName(instrument, note, variant),
					SoundSource.RECORDS, sound.getVolume() * FoleyClient.gain(), 1f, sound.getX(), sound.getY(),
					sound.getZ()));
			PLAYS.incrementAndGet();
		}
		return true;
	}
}
