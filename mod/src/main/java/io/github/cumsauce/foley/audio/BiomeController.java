package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Holder;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.level.biome.Biome;
import net.minecraft.world.level.biome.Biomes;
import org.jspecify.annotations.Nullable;

/**
 * Biome-specific ambient beds (M19): desert hiss, mushroom hum, lush/dripstone
 * cave colors, deep dark subsonic drone, snow hush, mangrove insect drone. One
 * streaming bed through {@link SurroundBeds}, sampled every {@value #SAMPLE_EVERY}
 * ticks. The Java side picks the kind from the listener's biome and starts a
 * new bed when the biome changes.
 */
public final class BiomeController {
	private static final int SAMPLE_EVERY = 5;
	/** How far round to look for mangrove leaves, blocks. */
	private static final int MANGROVE_RADIUS = 24;
	/** Below this temperature, °C, snow's hush is silent. */
	private static final float SNOW_OFF = 5f;
	/** Below this temperature, °C, snow's hush is at full intensity. */
	private static final float SNOW_FULL = -10f;

	private final SurroundBeds<BiomeScene> beds;
	private BiomeScene scene = BiomeScene.SILENT;
	private int ticks;

	public BiomeController(NativeBridge bridge) {
		this.beds = new SurroundBeds<>("biome", SoundSource.AMBIENT, bridge, NativeBridge.BED_BIOME);
	}

	/** The current scene. */
	public BiomeScene scene() {
		return scene;
	}

	/** One of its beds, if it's playing. */
	public @Nullable BedSound<BiomeScene> sound() {
		return beds.any();
	}

	/** Ticks the biome bed. Client thread. */
	public void tick(Minecraft minecraft) {
		if (minecraft.level == null || minecraft.player == null
				|| !minecraft.level.dimensionType().hasSkyLight()) {
			scene = BiomeScene.SILENT;
			beds.stop(minecraft);
			return;
		}
		if (ticks++ % SAMPLE_EVERY != 0) return;
		BiomeScene sampled = sample(minecraft);
		scene = sampled;
		if (sampled.gain() <= 0f) {
			beds.stop(minecraft);
			return;
		}
		// A single surround bed, weighted equally (the bed is a point source at the ear,
		// so the four-corner surround is mostly redundant for an omnipresent ambient, but
		// matches the rain/wind pattern in SurroundBeds).
		beds.keep(minecraft, scene, new float[] {1f, 1f, 1f, 1f});
	}

	private static BiomeScene sample(Minecraft minecraft) {
		var player = minecraft.player;
		if (player == null) return BiomeScene.SILENT;
		var level = minecraft.level;
		if (level == null) return BiomeScene.SILENT;
		Holder<Biome> biome = level.getBiome(BlockPos.containing(player.getEyePosition()));
		float celsius = 20 * biome.value().getBaseTemperature();
		int kind = -1;
		float intensity = 0f;
		float sub = 0f;
		// Desert and badlands share a bed; bamboo_jungle uses the jungle/lush bed via the
		// LUSH branch below (it's not strictly a desert, but a jungle).
		if (biome.is(Biomes.DESERT) || biome.is(Biomes.BADLANDS) || biome.is(Biomes.ERODED_BADLANDS)
				|| biome.is(Biomes.WOODED_BADLANDS)) {
			if (!FoleyClient.wants(FoleyConfig::desertBeds)) return BiomeScene.SILENT;
			kind = BiomeScene.DESERT;
			intensity = 1f;
			// `sub` = vortex rings per 30 s, capped at 2; only when U > 8 m/s.
			var wind = FoleyClient.wind();
			float speed = wind == null ? 0f : wind.scene().speed();
			sub = speed > 8f ? Math.min(2f, (speed - 8f) * 0.5f) : 0f;
		} else if (biome.is(Biomes.MUSHROOM_FIELDS)) {
			if (!FoleyClient.wants(FoleyConfig::mushroomHum)) return BiomeScene.SILENT;
			kind = BiomeScene.MUSHROOM;
			intensity = 1f;
		} else if (biome.is(Biomes.LUSH_CAVES)) {
			if (!FoleyClient.wants(FoleyConfig::caveColors)) return BiomeScene.SILENT;
			kind = BiomeScene.LUSH;
			intensity = 1f;
		} else if (biome.is(Biomes.DRIPSTONE_CAVES)) {
			if (!FoleyClient.wants(FoleyConfig::caveColors)) return BiomeScene.SILENT;
			kind = BiomeScene.DRIPSTONE;
			intensity = 1f;
		} else if (biome.is(Biomes.DEEP_DARK)) {
			if (!FoleyClient.wants(FoleyConfig::deepDark)) return BiomeScene.SILENT;
			kind = BiomeScene.DEEP_DARK;
			intensity = 1f;
			// `sub` = scuttle count per 30 s, capped at 2.
			sub = 1f;
		} else if (biome.is(Biomes.ICE_SPIKES) || biome.is(Biomes.FROZEN_OCEAN)
				|| biome.is(Biomes.DEEP_FROZEN_OCEAN) || biome.is(Biomes.SNOWY_BEACH)
				|| biome.is(Biomes.SNOWY_TAIGA) || biome.is(Biomes.SNOWY_PLAINS)
				|| biome.is(Biomes.SNOWY_SLOPES) || biome.is(Biomes.FROZEN_RIVER)
				|| biome.is(Biomes.GROVE) || biome.is(Biomes.JAGGED_PEAKS)
				|| biome.is(Biomes.FROZEN_PEAKS)) {
			if (!FoleyClient.wants(FoleyConfig::snowyHush)) return BiomeScene.SILENT;
			// Intensity ramps with celsius: full at SNOW_FULL, off at SNOW_OFF.
			intensity = celsius < SNOW_OFF
					? Math.max(0f, 1f - (celsius - SNOW_FULL) / (SNOW_OFF - SNOW_FULL))
					: 0f;
			if (intensity <= 0f) return BiomeScene.SILENT;
			kind = BiomeScene.SNOW;
			// `sub` = leaf exposure, 0 to 1 (0 open, 1 under trees).
			var wind = FoleyClient.wind();
			sub = wind == null ? 0f : Math.min(1f, wind.scene().leaves());
		} else if (biome.is(Biomes.MANGROVE_SWAMP)) {
			if (!FoleyClient.wants(FoleyConfig::mangroveDrone)) return BiomeScene.SILENT;
			// Count mangrove leaves in a 24-block radius around the player.
			int leaves = MangroveLeaves.count(level, BlockPos.containing(player.getEyePosition()), MANGROVE_RADIUS);
			kind = BiomeScene.MANGROVE;
			intensity = 1f;
			sub = leaves;
		}
		if (kind < 0) return BiomeScene.SILENT;
		// Apply the umbrella gate last (so a player can mute a single biome while
		// keeping the umbrella on).
		if (!FoleyClient.wants(FoleyConfig::biomeBeds)) return BiomeScene.SILENT;
		return new BiomeScene(kind, intensity, sub, 1f);
	}
}
