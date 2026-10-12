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
 * The rain round the listener, synthesized as it plays (M1): an endless stream the game reads like any streaming sound,
 * through Fabric's {@code getAudioStream}, so it plays through the game's own OpenAL channels. Around the listener, not
 * from a point. {@link #setScene} keeps it following the weather and what the rain lands on.
 */
public final class RainSound extends AbstractSoundInstance {
	static final AudioFormat FORMAT = new AudioFormat(NativeBridge.SAMPLE_RATE, 16, 1, true, false);
	private static final SoundEvent EVENT = SoundEvent.createVariableRangeEvent(
			Identifier.fromNamespaceAndPath("foley", "rain"));

	private final NativeBridge bridge;
	private volatile RainScene scene;
	private volatile long rendered;
	private Stream stream;

	public RainSound(NativeBridge bridge, RainScene scene) {
		super(EVENT, SoundSource.WEATHER, SoundInstance.createUnseededRandom());
		this.bridge = bridge;
		this.scene = scene;
		this.relative = true;
		this.attenuation = SoundInstance.Attenuation.NONE;
	}

	/** Samples rendered so far. For the client gametest. */
	public long rendered() {
		return rendered;
	}

	/** The rain to make from now on. Called on the client thread. */
	public synchronized void setScene(RainScene scene) {
		this.scene = scene;
		if (stream != null) stream.apply(scene);
	}

	public RainScene scene() {
		return scene;
	}

	@Override
	public synchronized CompletableFuture<AudioStream> getAudioStream(SoundBufferLibrary loader, Identifier id,
			boolean repeat) {
		stream = new Stream(bridge.rainCreate(random.nextLong()));
		stream.apply(scene);
		return CompletableFuture.completedFuture(stream);
	}

	/** The native bed, read a buffer at a time on the game's stream thread. */
	private final class Stream implements AudioStream {
		private final MemorySegment rain;
		private boolean closed;

		Stream(MemorySegment rain) {
			this.rain = rain;
		}

		synchronized void apply(RainScene scene) {
			if (!closed) bridge.rainSetScene(rain, scene.rate(), scene.surfaces(), scene.roof(), scene.height());
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
