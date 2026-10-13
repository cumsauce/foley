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

	private final Slot<Surroundings.Water> water;
	private final Slot<Surroundings.Fire> fire;
	private final Slot<Surroundings.Lava> lava;
	private Surroundings around = Surroundings.NONE;
	private int ticks;

	public PlacedBeds(NativeBridge bridge) {
		water = new Slot<>(bridge, "water", SoundSource.AMBIENT, NativeBridge.BED_WATER);
		fire = new Slot<>(bridge, "fire", SoundSource.BLOCKS, NativeBridge.BED_FIRE);
		lava = new Slot<>(bridge, "lava", SoundSource.BLOCKS, NativeBridge.BED_LAVA);
	}

	public Surroundings around() {
		return around;
	}

	public @Nullable BedSound<Surroundings.Water> water() {
		return water.sound;
	}

	public @Nullable BedSound<Surroundings.Fire> fire() {
		return fire.sound;
	}

	public @Nullable BedSound<Surroundings.Lava> lava() {
		return lava.sound;
	}

	public void tick(Minecraft minecraft) {
		if (minecraft.level == null || minecraft.player == null) {
			around = Surroundings.NONE;
			water.stop(minecraft);
			fire.stop(minecraft);
			lava.stop(minecraft);
			return;
		}
		if (ticks++ % SAMPLE_EVERY != 0) return;
		Vec3 eye = minecraft.player.getEyePosition();
		around = Surroundings.sample(minecraft.level, eye);
		// With Earshot, a bed fades with distance as any sound does, so it also gets the reverb, air and occlusion
		// a sound that far off would: it plays at its effective distance, in its direction, at full gain.
		boolean shaped = FoleyClient.earshot();
		Surroundings.Water w = around.water();
		Surroundings.Fire f = around.fire();
		Surroundings.Lava l = around.lava();
		water.keep(minecraft, shaped ? new Surroundings.Water(w.flow(), w.falls(), w.gain() > 0 ? 1 : 0, w.at()) : w,
				w.heard() && FoleyClient.wants(FoleyConfig::water), heardAt(eye, w.at(), w.gain(), shaped), shaped);
		fire.keep(minecraft, shaped ? new Surroundings.Fire(f.size(), f.gain() > 0 ? 1 : 0, f.at()) : f,
				f.heard() && FoleyClient.wants(FoleyConfig::fire), heardAt(eye, f.at(), f.gain(), shaped), shaped);
		lava.keep(minecraft, shaped ? new Surroundings.Lava(l.area(), l.gain() > 0 ? 1 : 0, l.at()) : l,
				l.heard() && FoleyClient.wants(FoleyConfig::lava), heardAt(eye, l.at(), l.gain(), shaped), shaped);
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

	/** One placed bed, started when there's something to hear, moved as that moves, stopped when it's gone. */
	private static final class Slot<S extends Scene> {
		private final NativeBridge bridge;
		private final String name;
		private final SoundSource source;
		private final int kind;
		@Nullable BedSound<S> sound;

		Slot(NativeBridge bridge, String name, SoundSource source, int kind) {
			this.bridge = bridge;
			this.name = name;
			this.source = source;
			this.kind = kind;
		}

		void keep(Minecraft minecraft, S scene, boolean heard, Vec3 at, boolean attenuated) {
			if (!heard) {
				stop(minecraft);
				return;
			}
			if (sound == null || !minecraft.getSoundManager().isActive(sound)) {
				sound = new BedSound<>(name, source, bridge, kind, scene).place(at, attenuated);
				minecraft.getSoundManager().play(sound);
			} else {
				sound.place(at, attenuated).setScene(scene);
			}
		}

		void stop(Minecraft minecraft) {
			if (sound != null) minecraft.getSoundManager().stop(sound);
			sound = null;
		}
	}
}
