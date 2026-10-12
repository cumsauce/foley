package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.natives.NativeBridge;
import java.lang.foreign.MemorySegment;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.concurrent.CompletableFuture;
import javax.sound.sampled.AudioFormat;
import net.minecraft.client.resources.sounds.AbstractSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.client.resources.sounds.TickableSoundInstance;
import net.minecraft.client.sounds.AudioStream;
import net.minecraft.client.sounds.SoundBufferLibrary;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.phys.Vec3;

/**
 * A bed (rain, wind, water, fire, lava): a native synth rendered as it plays, an endless stream the game reads like
 * any streaming sound, through Fabric's {@code getAudioStream}, so it plays through the game's own OpenAL channels.
 * Either all round the listener, or {@linkplain #place placed} where the sound comes from (and moved as that changes),
 * so it pans and Earshot occludes it. Distance is in the scene's gain, not OpenAL's falloff. {@link #setScene} keeps it
 * following the world.
 *
 * @param <S> what the bed is told about the world
 */
public final class BedSound<S extends Scene> extends AbstractSoundInstance implements TickableSoundInstance {
	static final AudioFormat FORMAT = new AudioFormat(NativeBridge.SAMPLE_RATE, 16, 1, true, false);

	private final NativeBridge bridge;
	private final int kind;
	private volatile S scene;
	private volatile long rendered;
	private Stream stream;

	/**
	 * @param name its sound event, {@code foley:<name>} in sounds.json
	 * @param kind its native kind, {@code NativeBridge.BED_*}
	 */
	public BedSound(String name, SoundSource source, NativeBridge bridge, int kind, S scene) {
		super(SoundEvent.createVariableRangeEvent(Identifier.fromNamespaceAndPath("foley", name)), source,
				SoundInstance.createUnseededRandom());
		this.bridge = bridge;
		this.kind = kind;
		this.scene = scene;
		this.relative = true;
		this.attenuation = SoundInstance.Attenuation.NONE;
	}

	/** Places it at {@code at} in the world, from now on. Called on the client thread. */
	public BedSound<S> place(Vec3 at) {
		relative = false;
		x = at.x;
		y = at.y;
		z = at.z;
		return this;
	}

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
	public boolean isStopped() {
		return false;
	}

	@Override
	public void tick() {}

	@Override
	public synchronized CompletableFuture<AudioStream> getAudioStream(SoundBufferLibrary loader, Identifier id,
			boolean repeat) {
		stream = new Stream(bridge.bedCreate(kind, random.nextLong()));
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
			if (!closed) bridge.bedSetScene(bed, scene.params());
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
			bridge.bedRender(bed, MemorySegment.ofBuffer(buffer));
			rendered += bytes / 2;
			return buffer;
		}

		@Override
		public synchronized void close() {
			if (closed) return;
			closed = true;
			bridge.bedDestroy(bed);
		}
	}
}
