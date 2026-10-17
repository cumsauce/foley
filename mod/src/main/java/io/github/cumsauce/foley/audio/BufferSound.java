package io.github.cumsauce.foley.audio;

import com.mojang.blaze3d.audio.SoundBuffer;
import java.io.ByteArrayInputStream;
import java.io.InputStream;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.concurrent.CompletableFuture;
import javax.sound.sampled.AudioFormat;
import net.minecraft.client.resources.sounds.AbstractSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.client.sounds.AudioStream;
import net.minecraft.client.sounds.WeighedSoundEvents;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundSource;
import net.minecraft.util.valueproviders.ConstantFloat;

/**
 * A pre-rendered PCM buffer played as a one-shot through the game's sound system. Used
 * by the M21 active-structures one-shots (boat bubble puffs, minecart rumble, shulker
 * puff): the synth is rendered by the native code into PCM and here played at the
 * world's position. The event identifier is only there to name the sound; the PCM is
 * wrapped as a {@link SoundBuffer} and streamed in through Fabric's {@code getAudioStream}.
 */
public final class BufferSound extends AbstractSoundInstance {
	private final short[] pcm;
	private final AudioFormat format;

	public BufferSound(Identifier event, float volume, double x, double y, double z, short[] pcm) {
		super(event, SoundSource.BLOCKS, SoundInstance.createUnseededRandom());
		this.pcm = pcm;
		this.format = BedSound.FORMAT;
		this.volume = volume;
		this.x = x;
		this.y = y;
		this.z = z;
		this.attenuation = SoundInstance.Attenuation.LINEAR;
	}

	@Override
	public WeighedSoundEvents getOrResolve(net.minecraft.client.sounds.SoundManager manager) {
		if (this.soundEvent == null) {
			WeighedSoundEvents events = new WeighedSoundEvents(this.identifier, null);
			events.addSound(new net.minecraft.client.resources.sounds.Sound(this.identifier, ConstantFloat.of(1f),
					ConstantFloat.of(1f), 1, net.minecraft.client.resources.sounds.Sound.Type.FILE, false, false, 16));
			this.soundEvent = events;
			this.sound = events.getSound(this.random);
		}
		return this.soundEvent;
	}

	@Override
	public CompletableFuture<AudioStream> getAudioStream(net.minecraft.client.sounds.SoundBufferLibrary loader,
			Identifier id, boolean repeat) {
		// A direct buffer has no backing array, so the stream reads a heap copy of the same PCM.
		byte[] heap = new byte[pcm.length * 2];
		ByteBuffer.wrap(heap).order(ByteOrder.LITTLE_ENDIAN).asShortBuffer().put(pcm);
		ByteBuffer bytes = ByteBuffer.allocateDirect(heap.length).order(ByteOrder.LITTLE_ENDIAN);
		bytes.put(heap).flip();
		SoundBuffer buffer = new SoundBuffer(bytes, format);
		InputStream in = new ByteArrayInputStream(heap);
		return CompletableFuture.completedFuture(new BufferStream(in, format));
	}

	private static final class BufferStream implements AudioStream {
		private final InputStream in;
		private final AudioFormat format;

		BufferStream(InputStream in, AudioFormat format) {
			this.in = in;
			this.format = format;
		}

		@Override
		public java.nio.ByteBuffer read(int capacity) {
			try {
				byte[] data = in.readNBytes(capacity);
				if (data.length == 0) return null;
				ByteBuffer buf = ByteBuffer.allocateDirect(data.length).order(ByteOrder.nativeOrder());
				buf.put(data).flip();
				return buf;
			} catch (java.io.IOException e) {
				return null;
			}
		}

		@Override
		public AudioFormat getFormat() {
			return format;
		}

		@Override
		public void close() {
			try {
				in.close();
			} catch (java.io.IOException e) {
				// ignore
			}
		}
	}
}
