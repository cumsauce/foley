package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.natives.NativeBridge;
import java.lang.foreign.MemorySegment;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import javax.sound.sampled.AudioFormat;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.AbstractSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.client.resources.sounds.TickableSoundInstance;
import net.minecraft.client.sounds.AudioStream;
import net.minecraft.client.sounds.SoundBufferLibrary;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

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
	/** A flash made before the game opened the stream. */
	private record Strike(float x, float y, float z, long seed) {}

	private final List<Strike> strikes = new ArrayList<>();

	/**
	 * @param name its sound event, {@code foley:<name>} in sounds.json
	 * @param kind its native kind, {@code NativeBridge.BED_*}
	 */
	public BedSound(String name, SoundSource source, NativeBridge bridge, int kind, S scene) {
		super(Sounds.event(name), source,
				SoundInstance.createUnseededRandom());
		this.bridge = bridge;
		this.kind = kind;
		this.scene = scene;
		this.relative = true;
		this.attenuation = SoundInstance.Attenuation.NONE;
		this.volume = FoleyClient.gain();
	}

	/** Where it plays from relative to the ear (world axes), following the player, or null. */
	private @Nullable Vec3 offset;
	/** How loud, against Foley's volume: the weight it's heading for, and where it's got to. */
	private float weight = 1;
	private float heardWeight = 1;

	/**
	 * Plays it {@code offset} from the player's ear (in world axes, so it stays put as they turn), at {@code weight}
	 * times Foley's volume. It is head-relative, its offset turned into the head's frame each tick: not a point in the
	 * world, so Earshot doesn't occlude or reroute it as that point passes through trees and hills, which crackled.
	 * Client thread.
	 */
	public BedSound<S> follow(Vec3 offset, float weight) {
		this.offset = offset;
		this.weight = weight;
		this.heardWeight = weight;
		relative = true;
		attenuation = SoundInstance.Attenuation.NONE;
		this.volume = FoleyClient.gain() * weight;
		turn();
		return this;
	}

	/** Sets the head-relative position from the world offset and where the player faces. */
	private void turn() {
		var player = Minecraft.getInstance().player;
		if (player == null || offset == null) return;
		double yaw = Math.toRadians(player.getYRot());
		// The player faces (-sin yaw, 0, cos yaw); their right is (-cos yaw, 0, -sin yaw). OpenAL's head frame has
		// +x to the right and -z ahead.
		double right = offset.x * -Math.cos(yaw) + offset.z * -Math.sin(yaw);
		double ahead = offset.x * -Math.sin(yaw) + offset.z * Math.cos(yaw);
		moveTo(new Vec3(right, offset.y, -ahead));
	}

	/** Heads for {@code weight} times Foley's volume, over a second or so. Client thread. */
	public void weigh(float weight) {
		this.weight = weight;
	}

	private void moveTo(Vec3 at) {
		x = at.x;
		y = at.y;
		z = at.z;
	}

	/** Places it at {@code at} in the world, from now on. Called on the client thread. */
	public BedSound<S> place(Vec3 at) {
		return place(at, false);
	}

	/**
	 * Places it at {@code at} in the world, from now on; {@code attenuated}, it fades with distance as other sounds do
	 * (for Earshot to shape), rather than by its scene's gain. Called on the client thread.
	 */
	public BedSound<S> place(Vec3 at, boolean attenuated) {
		relative = false;
		attenuation = attenuated ? SoundInstance.Attenuation.LINEAR : SoundInstance.Attenuation.NONE;
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

	/** A flash at ({@code x}, {@code y}, {@code z}) blocks from the listener, for a thunder bed. Client thread. */
	public synchronized void strike(float x, float y, float z, long seed) {
		if (stream != null) {
			stream.strike(x, y, z, seed);
		} else {
			strikes.add(new Strike(x, y, z, seed));
		}
	}

	/** Starts even at no volume (Foley's or its category's turned down), to be heard once it's turned up again. */
	@Override
	public boolean canStartSilent() {
		return true;
	}

	@Override
	public boolean isStopped() {
		return false;
	}

	/** Follows Foley's volume setting, its weight, and the player if it follows them. */
	@Override
	public void tick() {
		heardWeight += (weight - heardWeight) * 0.05f;
		volume = FoleyClient.gain() * heardWeight;
		if (offset != null) turn();
	}

	@Override
	public synchronized CompletableFuture<AudioStream> getAudioStream(SoundBufferLibrary loader, Identifier id,
			boolean repeat) {
		stream = new Stream(bridge.bedCreate(kind, random.nextLong()));
		stream.apply(scene);
		for (Strike strike : strikes) {
			stream.strike(strike.x(), strike.y(), strike.z(), strike.seed());
		}
		strikes.clear();
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

		synchronized void strike(float x, float y, float z, long seed) {
			if (!closed) bridge.bedStrike(bed, x, y, z, seed);
		}

		@Override
		public AudioFormat getFormat() {
			return FORMAT;
		}

		/**
		 * Reused: the game uploads each buffer to OpenAL before it reads the next. Sized for the second the game reads at
		 * a time, so reading doesn't allocate.
		 */
		private ByteBuffer scratch = ByteBuffer.allocateDirect(2 * NativeBridge.SAMPLE_RATE).order(ByteOrder.LITTLE_ENDIAN);

		@Override
		public synchronized ByteBuffer read(int size) {
			int bytes = size & ~1;
			if (scratch.capacity() < bytes) {
				scratch = ByteBuffer.allocateDirect(bytes).order(ByteOrder.LITTLE_ENDIAN);
			}
			ByteBuffer buffer = scratch.clear().limit(bytes);
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
