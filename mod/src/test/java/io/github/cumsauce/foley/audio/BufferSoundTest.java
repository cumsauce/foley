package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.Arrays;
import net.minecraft.client.sounds.AudioStream;
import net.minecraft.resources.Identifier;
import org.junit.jupiter.api.Test;

/**
 * A one-shot's stream must open from the direct buffer: the PCM goes in via a short-buffer view, and the stream reads
 * a heap copy — a direct buffer has no backing array, so {@code array()} on it always threw.
 */
class BufferSoundTest {
	@Test
	void bufferSoundOpensAStream() throws Exception {
		short[] pcm = {0, 1, -1, Short.MAX_VALUE, Short.MIN_VALUE, 1234};
		Identifier event = Identifier.fromNamespaceAndPath("foley", "test");
		BufferSound sound = new BufferSound(event, 1f, 0, 0, 0, pcm);
		AudioStream stream = sound.getAudioStream(null, event, false).get();
		assertNotNull(stream);
		assertEquals(BedSound.FORMAT.getSampleRate(), stream.getFormat().getSampleRate());
		assertEquals(BedSound.FORMAT.getSampleSizeInBits(), stream.getFormat().getSampleSizeInBits());
		// The stream hands back the same PCM it was given, little-endian, sample for sample.
		byte[] expected = new byte[pcm.length * 2];
		ByteBuffer.wrap(expected).order(ByteOrder.LITTLE_ENDIAN).asShortBuffer().put(pcm);
		ByteBuffer first = stream.read(expected.length);
		assertNotNull(first);
		byte[] actual = new byte[first.remaining()];
		first.duplicate().get(actual);
		assertEquals(Arrays.toString(expected), Arrays.toString(actual));
		assertNull(stream.read(expected.length), "end of stream");
		stream.close();
	}
}
