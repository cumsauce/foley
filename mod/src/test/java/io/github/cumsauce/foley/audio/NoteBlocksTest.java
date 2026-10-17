package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;

import net.minecraft.resources.Identifier;
import org.junit.jupiter.api.Test;

/** The note block table: the instrument order the native synth knows, and the buffers named from it. */
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
}
