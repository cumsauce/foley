package io.github.cumsauce.foley.audio;

import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.core.BlockPos;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.NoteBlock;
import net.minecraft.world.level.block.state.BlockState;
import java.util.List;
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

	private NoteBlocks() {}

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
		float pitch = sound.getPitch();
		int heard = pitch > 0 ? Math.round(12f + 12f * (float) (Math.log(pitch) / Math.log(2))) : 12;
		return Math.max(0, Math.min(24, heard));
	}
}
