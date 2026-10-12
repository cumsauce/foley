package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.LightningBolt;
import net.minecraft.world.phys.Vec3;
import org.jspecify.annotations.Nullable;

/**
 * Makes the thunder of every flash (M3), replacing the game's. It strikes when a lightning bolt appears, at the
 * moment of the flash, so the thunder arrives as late as sound takes from each part of the channel, whether or not
 * Earshot (which delays the game's own thunder) is there; the game's thunder sounds are then skipped. A thunder bed
 * plays while any thunder is still rumbling.
 */
public final class ThunderController {
	private static final int SAMPLE_EVERY = 10;
	/** The longest thunder rolls on for some 15 s; the bed stops this many ticks after the last flash. */
	private static final int LINGER = 20 * 25;

	private final NativeBridge bridge;
	private @Nullable BedSound<ThunderScene> sound;
	private int strikes;
	private int quietFor;

	public ThunderController(NativeBridge bridge) {
		this.bridge = bridge;
	}

	/** Whether Foley makes the thunder, so the game's own thunder is skipped. */
	public boolean replacing() {
		return io.github.cumsauce.foley.FoleyClient.wants(io.github.cumsauce.foley.config.FoleyConfig::thunder);
	}

	public @Nullable BedSound<ThunderScene> sound() {
		return sound;
	}

	/** Flashes heard so far. For the client gametest. */
	public int strikes() {
		return strikes;
	}

	/** A lightning bolt appeared on the client: its flash is now. */
	public void entityLoaded(Entity entity) {
		if (entity instanceof LightningBolt bolt) strike(Minecraft.getInstance(), bolt.position(), bolt.getUUID()
				.getLeastSignificantBits());
	}

	public void strike(Minecraft minecraft, Vec3 at, long seed) {
		if (minecraft.level == null || minecraft.player == null || !replacing()) return;
		Vec3 eye = minecraft.player.getEyePosition();
		if (sound == null || !minecraft.getSoundManager().isActive(sound)) {
			sound = new BedSound<>("thunder", SoundSource.WEATHER, bridge, NativeBridge.BED_THUNDER, scene(minecraft));
			minecraft.getSoundManager().play(sound);
		}
		sound.strike((float) (at.x - eye.x), (float) (at.y - eye.y), (float) (at.z - eye.z), seed);
		strikes++;
		quietFor = 0;
	}

	public void tick(Minecraft minecraft) {
		if (sound == null) return;
		if (minecraft.level == null || ++quietFor > LINGER) {
			minecraft.getSoundManager().stop(sound);
			sound = null;
			return;
		}
		if (quietFor % SAMPLE_EVERY == 0) sound.setScene(scene(minecraft));
	}

	private static ThunderScene scene(Minecraft minecraft) {
		var wind = io.github.cumsauce.foley.FoleyClient.wind();
		return ThunderScene.sample(minecraft, wind == null ? 1 : wind.scene().exposure());
	}
}
