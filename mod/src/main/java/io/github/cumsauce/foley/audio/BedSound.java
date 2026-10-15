package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.natives.NativeBridge;
import java.lang.foreign.MemorySegment;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.List;
import java.util.Queue;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ConcurrentLinkedQueue;
import java.util.concurrent.atomic.AtomicReference;
import javax.sound.sampled.AudioFormat;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.AbstractSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.client.resources.sounds.TickableSoundInstance;
import net.minecraft.client.sounds.AudioStream;
import net.minecraft.client.sounds.SoundBufferLibrary;
import net.minecraft.client.sounds.SoundManager;
import net.minecraft.client.sounds.WeighedSoundEvents;
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
	/** Set on an addon's bed, which has no sounds.json entry and resolves itself. */
	private boolean selfResolving;
	/** Set on an addon's bed told to stop; the game retires it on its next tick. */
	private boolean stopped;
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

	/** An addon's bed: it has no sounds.json entry, so it resolves itself to a streaming sound (F2). */
	static BedSound<AddonBeds.AddonScene> addon(String name, NativeBridge bridge, int kind, float[] scene) {
		BedSound<AddonBeds.AddonScene> sound = new BedSound<>(name, SoundSource.AMBIENT, bridge, kind,
				new AddonBeds.AddonScene(scene));
		sound.selfResolving = true;
		return sound;
	}

	/** The native kind this bed renders. */
	int nativeKind() {
		return kind;
	}

	/** Tells the game to retire the bed; an addon's only. */
	void retire() {
		stopped = true;
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
		return stopped;
	}

	/** The bypass silences everything Foley makes, addons' beds too, till it's off again. */
	@Override
	public void tick() {
		heardWeight += (weight - heardWeight) * 0.05f;
		volume = FoleyClient.gain() * heardWeight * (FoleyClient.bypassed() ? 0f : 1f);
		if (offset != null) turn();
	}

	@Override
	public WeighedSoundEvents getOrResolve(SoundManager manager) {
		if (!selfResolving) return super.getOrResolve(manager);
		if (this.soundEvent == null) {
			// Any location: the stream comes from the bed itself, never from a file.
			WeighedSoundEvents events = new WeighedSoundEvents(this.identifier, null);
			events.addSound(new net.minecraft.client.resources.sounds.Sound(this.identifier,
					net.minecraft.util.valueproviders.ConstantFloat.of(1f),
					net.minecraft.util.valueproviders.ConstantFloat.of(1f), 1,
					net.minecraft.client.resources.sounds.Sound.Type.FILE, true, false, 16));
			this.soundEvent = events;
			this.sound = events.getSound(this.random);
		}
		return this.soundEvent;
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

	/**
	 * The native bed, read a buffer at a time on the game's stream thread. Scenes and strikes are handed over without
	 * a lock and taken up before the next buffer, so the client thread never waits on a render.
	 */
	private final class Stream implements AudioStream {
		private final MemorySegment bed;
		private boolean closed;
		private final AtomicReference<float @Nullable []> pending = new AtomicReference<>();
		private final Queue<Strike> flashes = new ConcurrentLinkedQueue<>();

		Stream(MemorySegment bed) {
			this.bed = bed;
		}

		void apply(S scene) {
			pending.set(scene.params());
		}

		void strike(float x, float y, float z, long seed) {
			flashes.add(new Strike(x, y, z, seed));
		}

		/** Takes up the latest scene and any flashes. Stream thread, holding the lock. */
		private void catchUp() {
			float[] params = pending.getAndSet(null);
			try {
				if (params != null) bridge.bedSetScene(bed, params);
				for (Strike s; (s = flashes.poll()) != null; ) bridge.bedStrike(bed, s.x(), s.y(), s.z(), s.seed());
			} catch (RuntimeException e) {
				FoleyClient.LOGGER.warn("Foley bed rejected its scene", e);
			}
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

		/**
		 * At most half a second a read. The game asks a second at a time and keeps four queued, so a change in the
		 * world was heard up to 4 s late; returning less, it queues four of these instead: 2 s at most, still well
		 * past any hitch it rides out.
		 */
		private static final int MOST_BYTES = NativeBridge.SAMPLE_RATE;

		@Override
		public synchronized ByteBuffer read(int size) {
			int bytes = Math.min(size, MOST_BYTES) & ~1;
			if (scratch.capacity() < bytes) {
				scratch = ByteBuffer.allocateDirect(bytes).order(ByteOrder.LITTLE_ENDIAN);
			}
			ByteBuffer buffer = scratch.clear().limit(bytes);
			if (closed) return buffer.limit(0);
			catchUp();
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
