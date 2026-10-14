package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * Keeps the water, fire and lava beds playing where they're heard, following the {@link Surroundings} twice a second.
 * They replace the game's own water, fire and lava sounds within reach (see {@link Replacements}).
 */
public final class PlacedBeds {
	private static final int SAMPLE_EVERY = 10;
	/**
	 * A bed's gain when Earshot fades it with distance: Earshot's inverse-distance law starts from 3 blocks, Foley's
	 * own from 2, so this keeps a bed as loud as it was.
	 */
	private static final float SHAPED_GAIN = 2f / 3;

	/** One bed per quarter round the listener, for each kind. */
	private final Parts<Surroundings.Water> water;
	private final Parts<Surroundings.Fire> fire;
	private final Parts<Surroundings.Lava> lava;
	private Surroundings around = Surroundings.NONE;
	private int ticks;

	public PlacedBeds(NativeBridge bridge) {
		water = new Parts<>(bridge, "water", SoundSource.AMBIENT, NativeBridge.BED_WATER);
		fire = new Parts<>(bridge, "fire", SoundSource.BLOCKS, NativeBridge.BED_FIRE);
		lava = new Parts<>(bridge, "lava", SoundSource.BLOCKS, NativeBridge.BED_LAVA);
	}

	public Surroundings around() {
		return around;
	}

	/** A water bed playing, if any (there may be one in each quarter). */
	public @Nullable BedSound<Surroundings.Water> water() {
		return water.any();
	}

	public @Nullable BedSound<Surroundings.Fire> fire() {
		return fire.any();
	}

	public @Nullable BedSound<Surroundings.Lava> lava() {
		return lava.any();
	}

	public void tick(Minecraft minecraft) {
		if (minecraft.level == null || minecraft.player == null) {
			around = Surroundings.NONE;
			water.stop(minecraft);
			fire.stop(minecraft);
			lava.stop(minecraft);
			return;
		}
		Vec3 eye = minecraft.player.getEyePosition();
		if (ticks++ % SAMPLE_EVERY != 0) {
			// Between samples, each bed glides towards where it's heard from now (with Earshot, at its effective
			// distance from where the ear is now): set only at each sample, it stepped twice a second.
			water.follow(eye);
			fire.follow(eye);
			lava.follow(eye);
			return;
		}
		around = Surroundings.sample(minecraft.level, eye);
		// With Earshot, a bed fades with distance as any sound does, so it also gets the reverb, air and occlusion
		// a sound that far off would: it plays at its effective distance, in its direction, at full gain.
		boolean shaped = FoleyClient.earshot();
		for (int part = 0; part < Surroundings.PARTS; part++) {
			Surroundings.Water w = around.waterParts()[part];
			Surroundings.Fire f = around.fireParts()[part];
			Surroundings.Lava l = around.lavaParts()[part];
			water.slots[part].keep(minecraft,
					w != null && shaped ? new Surroundings.Water(w.flow(), w.falls(), w.gain() > 0 ? SHAPED_GAIN : 0, w.at()) : w,
					w != null && w.heard() && FoleyClient.wants(FoleyConfig::water),
					eye, w == null ? eye : w.at(), w == null ? 0 : w.gain(), shaped);
			fire.slots[part].keep(minecraft,
					f != null && shaped ? new Surroundings.Fire(f.size(), f.gain() > 0 ? SHAPED_GAIN : 0, f.at()) : f,
					f != null && f.heard() && FoleyClient.wants(FoleyConfig::fire),
					eye, f == null ? eye : f.at(), f == null ? 0 : f.gain(), shaped);
			lava.slots[part].keep(minecraft,
					l != null && shaped ? new Surroundings.Lava(l.area(), l.gain() > 0 ? SHAPED_GAIN : 0, l.at()) : l,
					l != null && l.heard() && FoleyClient.wants(FoleyConfig::lava),
					eye, l == null ? eye : l.at(), l == null ? 0 : l.gain(), shaped);
		}
	}

	/**
	 * Where a bed plays from: its power-weighted centre, or, when distance is left to Earshot, the effective distance
	 * (2 / gain) out from the ear towards that centre, so water all round doesn't collapse into the listener's head.
	 */
	static Vec3 heardAt(Vec3 eye, Vec3 centre, float gain, boolean shaped) {
		Vec3 toward = centre.subtract(eye);
		if (!shaped || gain <= 0 || toward.lengthSqr() < 1e-4) return centre;
		double distance = Math.max(1, 2 / gain);
		return eye.add(toward.normalize().scale(distance));
	}

	/** A kind's beds, one per quarter round the listener. */
	private static final class Parts<S extends Scene> {
		@SuppressWarnings("unchecked")
		final Slot<S>[] slots = new Slot[Surroundings.PARTS];

		Parts(NativeBridge bridge, String name, SoundSource source, int kind) {
			for (int i = 0; i < slots.length; i++) slots[i] = new Slot<>(bridge, name, source, kind);
		}

		@Nullable BedSound<S> any() {
			for (Slot<S> slot : slots) {
				if (slot.sound != null) return slot.sound;
			}
			return null;
		}

		void stop(Minecraft minecraft) {
			for (Slot<S> slot : slots) slot.stop(minecraft);
		}

		void follow(Vec3 eye) {
			for (Slot<S> slot : slots) slot.follow(eye);
		}
	}

	/** One placed bed, started when there's something to hear, moved as that moves, stopped when it's gone. */
	private static final class Slot<S extends Scene> {
		private final NativeBridge bridge;
		private final String name;
		private final SoundSource source;
		private final int kind;
		@Nullable BedSound<S> sound;
		/** What it was last aimed at: the sound's centre, its gain, and whether Earshot shapes it. */
		private Vec3 centre = Vec3.ZERO;
		/** Where it plays from now, gliding to where it's aimed. */
		private Vec3 playsFrom = Vec3.ZERO;
		private float gain;
		private boolean shaped;

		Slot(NativeBridge bridge, String name, SoundSource source, int kind) {
			this.bridge = bridge;
			this.name = name;
			this.source = source;
			this.kind = kind;
		}

		void keep(Minecraft minecraft, @Nullable S scene, boolean heard, Vec3 eye, Vec3 centre, float gain,
				boolean attenuated) {
			if (!heard || scene == null) {
				stop(minecraft);
				return;
			}
			this.centre = centre;
			this.gain = gain;
			this.shaped = attenuated;
			if (sound == null || !minecraft.getSoundManager().isActive(sound)) {
				playsFrom = heardAt(eye, centre, gain, attenuated);
				sound = new BedSound<>(name, source, bridge, kind, scene).place(playsFrom, attenuated);
				minecraft.getSoundManager().play(sound);
			} else {
				sound.setScene(scene);
				follow(eye);
			}
		}

		/** Moves a bed a step towards where it's heard from, for the ear at {@code eye}: about a third of a second. */
		void follow(Vec3 eye) {
			if (sound == null) return;
			playsFrom = playsFrom.lerp(heardAt(eye, centre, gain, shaped), 0.15);
			sound.place(playsFrom, shaped);
		}

		void stop(Minecraft minecraft) {
			if (sound != null) minecraft.getSoundManager().stop(sound);
			sound = null;
		}
	}
}
