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
		around = Surroundings.sample(minecraft.level, minecraft.player.getEyePosition());
		water.keep(minecraft, around.water(), around.water().heard() && FoleyClient.wants(FoleyConfig::water),
				around.water().at());
		fire.keep(minecraft, around.fire(), around.fire().heard() && FoleyClient.wants(FoleyConfig::fire),
				around.fire().at());
		lava.keep(minecraft, around.lava(), around.lava().heard() && FoleyClient.wants(FoleyConfig::lava),
				around.lava().at());
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

		void keep(Minecraft minecraft, S scene, boolean heard, Vec3 at) {
			if (!heard) {
				stop(minecraft);
				return;
			}
			if (sound == null || !minecraft.getSoundManager().isActive(sound)) {
				sound = new BedSound<>(name, source, bridge, kind, scene).place(at);
				minecraft.getSoundManager().play(sound);
			} else {
				sound.place(at).setScene(scene);
			}
		}

		void stop(Minecraft minecraft) {
			if (sound != null) minecraft.getSoundManager().stop(sound);
			sound = null;
		}
	}
}
