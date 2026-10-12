package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.natives.NativeBridge;
import java.lang.foreign.MemorySegment;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.concurrent.CompletableFuture;
import javax.sound.sampled.AudioFormat;
import net.minecraft.client.resources.sounds.AbstractSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.client.sounds.AudioStream;
import net.minecraft.client.sounds.SoundBufferLibrary;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;

/**
 * A bed of rain on water, synthesized as it plays (M0 spike): an endless stream the game reads like any streaming
 * sound, through Fabric's {@code getAudioStream}, so it plays through the game's own OpenAL channels and Earshot shapes
 * it. Around the listener, not from a point.
 */
public final class RainSound extends AbstractSoundInstance {
	static final AudioFormat FORMAT = new AudioFormat(NativeBridge.SAMPLE_RATE, 16, 1, true, false);
	private static final SoundEvent EVENT = SoundEvent.createVariableRangeEvent(
			Identifier.fromNamespaceAndPath("foley", "rain"));

	private final NativeBridge bridge;
	private final float rate;
	private volatile long rendered;

	public RainSound(NativeBridge bridge, float rate) {
		super(EVENT, SoundSource.WEATHER, SoundInstance.createUnseededRandom());
		this.bridge = bridge;
		this.rate = rate;
		this.relative = true;
		this.attenuation = SoundInstance.Attenuation.NONE;
	}

	/** Samples rendered so far. For the client gametest. */
	public long rendered() {
		return rendered;
	}

	@Override
	public CompletableFuture<AudioStream> getAudioStream(SoundBufferLibrary loader, Identifier id, boolean repeat) {
		return CompletableFuture.completedFuture(new Stream(bridge.rainCreate(random.nextLong(), rate)));
	}

	/** The native bed, read a buffer at a time on the game's stream thread. */
	private final class Stream implements AudioStream {
		private final MemorySegment rain;
		private boolean closed;

		Stream(MemorySegment rain) {
			this.rain = rain;
		}

		@Override
		public AudioFormat getFormat() {
			return FORMAT;
		}

		@Override
		public synchronized ByteBuffer read(int size) {
			int bytes = size & ~1;
			ByteBuffer buffer = ByteBuffer.allocateDirect(bytes).order(ByteOrder.LITTLE_ENDIAN);
			if (closed) return buffer.limit(0);
			bridge.rainRender(rain, MemorySegment.ofBuffer(buffer));
			rendered += bytes / 2;
			return buffer;
		}

		@Override
		public synchronized void close() {
			if (closed) return;
			closed = true;
			bridge.rainDestroy(rain);
		}
	}
}
