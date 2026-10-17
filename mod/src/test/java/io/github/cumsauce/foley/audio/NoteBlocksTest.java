package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import org.junit.jupiter.api.Test;

/** The note block table: the instrument order the native synth knows, the buffers named from it, and the strike. */
class NoteBlocksTest {
	@Test
	void theInstrumentOrderMatchesTheIds() {
		assertEquals(16, NoteBlocks.INSTRUMENTS.size());
		assertEquals("harp", NoteBlocks.INSTRUMENTS.get(0));
		assertEquals("pling", NoteBlocks.INSTRUMENTS.get(15));
	}

	@Test
	void aVanillaNoteBlockIdYieldsItsInstrument() {
		assertEquals(0, NoteBlocks.instrument(Identifier.fromNamespaceAndPath("minecraft", "block.note_block.harp")));
		assertEquals(-1, NoteBlocks.instrument(Identifier.fromNamespaceAndPath("minecraft", "block.note_block.unknown")));
		assertEquals(-1, NoteBlocks.instrument(Identifier.fromNamespaceAndPath("minecraft", "block.stone.break")));
	}

	@Test
	void theBufferNameEncodesTheInstrumentAndNote() {
		assertEquals("foley:generated/note_0_12_0", NoteBlocks.bufferName(0, 12, 0).toString());
	}

	@Test
	void unknownNoteSoundIsLeftToVanilla() {
		assertFalse(NoteBlocks.replace(sound("block.note_block.unknown"), client()));
	}

	@Test
	void aNoteBlockSoundIsTaken() {
		assertTrue(NoteBlocks.replace(sound("block.note_block.harp"), client()));
	}

	@Test
	void unreadableNoteFallsBackToTheInstancePitch() {
		assertEquals(12, NoteBlocks.noteOf(soundWithPitch("block.note_block.harp", 1.0f), null));
	}

	/** The game's note block sound at that path, played at {@code pitch} from the origin. */
	private static SoundInstance sound(String path) {
		return soundWithPitch(path, 1.0f);
	}

	/** The same, at an explicit pitch, so a note can be heard without a block to read. */
	private static SoundInstance soundWithPitch(String path, float pitch) {
		SoundEvent event = SoundEvent.createVariableRangeEvent(Identifier.fromNamespaceAndPath("minecraft", path));
		return new SimpleSoundInstance(event, SoundSource.RECORDS, 1f, pitch, SoundInstance.createUnseededRandom(),
				0.0, 0.0, 0.0);
	}

	/** The client, as the tests see one: none, so blocks cannot be read and nothing plays. */
	private static Minecraft client() {
		return Minecraft.getInstance();
	}
}
