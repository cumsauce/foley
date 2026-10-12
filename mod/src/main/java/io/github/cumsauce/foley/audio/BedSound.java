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
 * A bed (rain, wind): a native synth rendered as it plays, an endless stream the game reads like any streaming sound,
 * through Fabric's {@code getAudioStream}, so it plays through the game's own OpenAL channels. Around the listener, not
 * from a point. {@link #setScene} keeps it following the world.
 *
 * @param <S> what the bed is told about the world
 */
abstract class BedSound<S> extends AbstractSoundInstance {
	static final AudioFormat FORMAT = new AudioFormat(NativeBridge.SAMPLE_RATE, 16, 1, true, false);

	final NativeBridge bridge;
	private volatile S scene;
	private volatile long rendered;
	private Stream stream;

	BedSound(String name, SoundSource source, NativeBridge bridge, S scene) {
		super(SoundEvent.createVariableRangeEvent(Identifier.fromNamespaceAndPath("foley", name)), source,
				SoundInstance.createUnseededRandom());
		this.bridge = bridge;
		this.scene = scene;
		this.relative = true;
		this.attenuation = SoundInstance.Attenuation.NONE;
	}

	/** Starts the native bed. */
	abstract MemorySegment create(long seed);

	/** Tells the native bed about {@code scene}. */
	abstract void apply(MemorySegment bed, S scene);

	/** Renders the next samples into {@code out}, native memory 2 bytes a sample. */
	abstract void render(MemorySegment bed, MemorySegment out);

	abstract void destroy(MemorySegment bed);

	/** Samples rendered so far. For the client gametest. */
	public long rendered() {
		return rendered;
	}

	/** What to make from now on. Called on the client thread. */
	public synchronized void setScene(S scene) {
		this.scene = scene;
		if (stream != null) stream.apply(scene);
	}

	public S scene() {
		return scene;
	}

	@Override
	public synchronized CompletableFuture<AudioStream> getAudioStream(SoundBufferLibrary loader, Identifier id,
			boolean repeat) {
		stream = new Stream(create(random.nextLong()));
		stream.apply(scene);
		return CompletableFuture.completedFuture(stream);
	}

	/** The native bed, read a buffer at a time on the game's stream thread. */
	private final class Stream implements AudioStream {
		private final MemorySegment bed;
		private boolean closed;

		Stream(MemorySegment bed) {
			this.bed = bed;
		}

		synchronized void apply(S scene) {
			if (!closed) BedSound.this.apply(bed, scene);
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
			render(bed, MemorySegment.ofBuffer(buffer));
			rendered += bytes / 2;
			return buffer;
		}

		@Override
		public synchronized void close() {
			if (closed) return;
			closed = true;
			destroy(bed);
		}
	}
}
