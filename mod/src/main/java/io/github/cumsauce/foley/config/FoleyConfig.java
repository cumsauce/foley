package io.github.cumsauce.foley.config;

import com.google.gson.GsonBuilder;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParseException;
import com.google.gson.JsonParser;
import com.google.gson.JsonPrimitive;
import io.github.cumsauce.foley.FoleyClient;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.NoSuchFileException;
import java.nio.file.Path;

/**
 * Foley's settings, kept in {@code config/foley/foley.json}. A missing or unreadable value falls back to its default,
 * so an old or hand-edited file still loads. Every sound Foley replaces can be given back to the game, and every one it
 * adds switched off.
 *
 * @param volume     Foley's sounds' volume, 0 to 100 %, under the game's own sliders
 * @param rain       Foley's rain rather than the game's
 * @param wind       Foley's wind (added)
 * @param thunder    Foley's thunder rather than the game's
 * @param water      Foley's streams and waterfalls rather than the game's water ambience
 * @param fire       Foley's fire rather than the game's
 * @param lava       Foley's lava rather than the game's
 * @param drips      Foley's dripstone drips rather than the game's
 * @param strikes    Foley's strikes and breaks under the game's block sounds (added)
 * @param footsteps  Foley's footsteps under the game's step sounds (added)
 * @param birds      birdsong (added)
 * @param crickets   crickets (added)
 * @param frogs      frogs (added)
 * @param minecarts  active structures (M21): minecart wheels and rush
 * @param doors      active structures (M21): door creaks
 * @param chests     active structures (M21): chest and shulker open
 * @param redstone   active structures (M21): redstone component state-change clicks
 * @param boats      active structures (M21): boat lapping at the hull
 * @param sculk      active structures (M21): sculk sensor ring
 * @param noteBlocks note block instruments (M23): Foley strikes its own rather than the game's
 * @param caveAtmosphere cave atmosphere (M12): ceiling drips and a sub-bass rumble underground; off by default
 * @param rarerSpecies rare birdsong (M13): Kori bustard, golden eagle, alpine chough (off by default)
 * @param underwater underwater ambience (M14): bubbles rising past the ear and a low pressure tone when the head is under
 * @param aeolian    aeolian whistles from iron bars, fences, lanterns, bells and bamboo (added)
 * @param biomeBeds  biome-specific beds (M19): umbrella toggle
 * @param desertBeds desert / badlands sand hiss
 * @param mushroomHum mushroom fields hum
 * @param caveColors lush and dripstone cave beds
 * @param deepDark   deep dark subsonic drone and scuttles
 * @param snowyHush  snow / ice / frozen ocean hush
 * @param mangroveDrone mangrove swamp insect drone
 * @param village    village ambience (M20): bell, murmur, smith and scattered animals
 * @param settling          subtle physical layers (M22): delayed settle tick after footsteps and block placement
 * @param mobBrushes        subtle physical layers (M22): sheep, spiders and chickens brushing through plants
 * @param peacefulBreathing subtle physical layers (M22): quiet breath from cows and sheep at night
 * @param ashDrift          subtle physical layers (M22): high-frequency bandpassed noise during blizzards and ash storms
 * @param voices     which mobs Foley voices (M6, M7), a bit each in {@code Voices.GROUPS}' order; none by default
 */
public record FoleyConfig(int volume, boolean rain, boolean wind, boolean thunder, boolean water, boolean fire,
		boolean lava, boolean drips, boolean strikes, boolean footsteps, boolean birds, boolean crickets,
		boolean frogs, boolean minecarts, boolean doors, boolean chests, boolean redstone, boolean boats,
		boolean sculk, boolean noteBlocks, boolean caveAtmosphere, boolean rarerSpecies, boolean underwater,
		boolean aeolian, boolean biomeBeds, boolean desertBeds, boolean mushroomHum,
		boolean caveColors, boolean deepDark, boolean snowyHush, boolean mangroveDrone,
		boolean village, boolean settling, boolean mobBrushes, boolean peacefulBreathing, boolean ashDrift,
		long voices) {
	public static final FoleyConfig DEFAULT = new FoleyConfig(100, true, true, true, true, true, true, true, true, true,
			true, true, true, true, true, true, true, true, true, true, false, false, true, true, true, true, true,
			true, true, true, true, true, true, true, true, true, 0);
	/** Everything on, every mob's voice too. */
	public static final FoleyConfig ALL = DEFAULT.withVoices(~0L).withCaveAtmosphere(true).withRarerSpecies(true)
			.withUnderwater(true).withAeolian(true);
	/** Everything off. */
	public static final FoleyConfig NONE = new FoleyConfig(100, false, false, false, false, false, false, false, false,
			false, false, false, false, false, false, false, false, false, false, false, false, false,
			false, false, false, false, false, false, false, false, false, false, false, false, false, false, 0);
	/** These settings with the mob voices in {@code mask} ({@code Voices.GROUPS}' bits). */
	public FoleyConfig withVoices(long mask) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village,
				settling, mobBrushes, peacefulBreathing, ashDrift, mask);
	}

	/** These settings with the wind on or off. */
	public FoleyConfig withWind(boolean on) {
		return new FoleyConfig(volume, rain, on, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village,
				settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with flowing water on or off. */
	public FoleyConfig withWater(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, on, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village,
				settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with the cave atmosphere on or off. */
	public FoleyConfig withCaveAtmosphere(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, on, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village, settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with {@code rarerSpecies} on or off. */
	public FoleyConfig withRarerSpecies(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, on, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village, settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with the underwater ambience on or off. */
	public FoleyConfig withUnderwater(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, on,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village, settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with the aeolian tones on or off. */
	public FoleyConfig withAeolian(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				on, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village, settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with biome-specific beds (M19) on or off. */
	public FoleyConfig withBiomeBeds(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, on, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village, settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with the desert sand hiss on or off. */
	public FoleyConfig withDesertBeds(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, on, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village, settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with the mushroom fields hum on or off. */
	public FoleyConfig withMushroomHum(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, on, caveColors, deepDark, snowyHush, mangroveDrone, village, settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with lush and dripstone cave colors on or off. */
	public FoleyConfig withCaveColors(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, on, deepDark, snowyHush, mangroveDrone, village, settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with the deep dark drone on or off. */
	public FoleyConfig withDeepDark(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, on, snowyHush, mangroveDrone, village, settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with the snow / ice hush on or off. */
	public FoleyConfig withSnowyHush(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, on, mangroveDrone, village, settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with the mangrove insect drone on or off. */
	public FoleyConfig withMangroveDrone(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, on, village, settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with the village ambience on or off. */
	public FoleyConfig withVillage(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, on, settling, mobBrushes, peacefulBreathing, ashDrift, voices);	}

	/** These settings with the settling tick on or off. */
	public FoleyConfig withSettling(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village,
				on, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with the mob brushes on or off. */
	public FoleyConfig withMobBrushes(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village,
				settling, on, peacefulBreathing, ashDrift, voices);
	}

	/** These settings with peaceful-mob breathing on or off. */
	public FoleyConfig withPeacefulBreathing(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village,
				settling, mobBrushes, on, ashDrift, voices);
	}

	/** These settings with ash and snow drift on or off. */
	public FoleyConfig withAshDrift(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, noteBlocks, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village,
				settling, mobBrushes, peacefulBreathing, on, voices);
	}

	/** These settings with the note block strikes on or off. */
	public FoleyConfig withNoteBlocks(boolean on) {
		return new FoleyConfig(volume, rain, wind, thunder, water, fire, lava, drips, strikes, footsteps, birds, crickets,
				frogs, minecarts, doors, chests, redstone, boats, sculk, on, caveAtmosphere, rarerSpecies, underwater,
				aeolian, biomeBeds, desertBeds, mushroomHum, caveColors, deepDark, snowyHush, mangroveDrone, village,
				settling, mobBrushes, peacefulBreathing, ashDrift, voices);
	}

	/** Whether Foley voices the group at {@code index} in {@code Voices.GROUPS}. */
	public boolean voice(int index) {
		return index >= 0 && index < 64 && (voices & (1L << index)) != 0;
	}

	/** Foley's volume as a gain, 0 to 1. */
	public float gain() {
		return Math.clamp(volume, 0, 100) / 100f;
	}

	/** The settings in {@code file}, or the defaults if there is none or it can't be read. */
	public static FoleyConfig load(Path file) {
		try {
			JsonElement json = JsonParser.parseString(Files.readString(file));
			return json.isJsonObject() ? fromJson(json.getAsJsonObject()) : DEFAULT;
		} catch (NoSuchFileException e) {
			return DEFAULT;
		} catch (IOException | JsonParseException e) {
			FoleyClient.LOGGER.warn("Couldn't read {}, so Foley uses its default settings", file, e);
			return DEFAULT;
		}
	}

	public void save(Path file) {
		try {
			Files.createDirectories(file.getParent());
			Files.writeString(file, new GsonBuilder().setPrettyPrinting().create().toJson(toJson()) + "\n");
		} catch (IOException e) {
			FoleyClient.LOGGER.warn("Couldn't save Foley's settings to {}", file, e);
		}
	}

	static FoleyConfig fromJson(JsonObject json) {
		FoleyConfig d = DEFAULT;
		int volume = json.get("volume") instanceof JsonPrimitive p && p.isNumber() ? Math.clamp(p.getAsInt(), 0, 100)
				: d.volume;
		return new FoleyConfig(volume, flag(json, "rain", d.rain), flag(json, "wind", d.wind),
				flag(json, "thunder", d.thunder), flag(json, "water", d.water), flag(json, "fire", d.fire),
				flag(json, "lava", d.lava), flag(json, "drips", d.drips), flag(json, "strikes", d.strikes),
				flag(json, "footsteps", d.footsteps), flag(json, "birds", d.birds), flag(json, "crickets", d.crickets),
				flag(json, "frogs", d.frogs), flag(json, "minecarts", d.minecarts), flag(json, "doors", d.doors),
				flag(json, "chests", d.chests), flag(json, "redstone", d.redstone), flag(json, "boats", d.boats),
				flag(json, "sculk", d.sculk), flag(json, "noteBlocks", d.noteBlocks),
				flag(json, "caveAtmosphere", d.caveAtmosphere),
				flag(json, "rarerSpecies", d.rarerSpecies), flag(json, "underwater", d.underwater),
				flag(json, "aeolian", d.aeolian), flag(json, "biomeBeds", d.biomeBeds),
				flag(json, "desertBeds", d.desertBeds), flag(json, "mushroomHum", d.mushroomHum),
				flag(json, "caveColors", d.caveColors), flag(json, "deepDark", d.deepDark),
				flag(json, "snowyHush", d.snowyHush), flag(json, "mangroveDrone", d.mangroveDrone),
				flag(json, "village", d.village), flag(json, "settling", d.settling),
				flag(json, "mob_brushes", d.mobBrushes), flag(json, "peaceful_breathing", d.peacefulBreathing),
				flag(json, "ash_drift", d.ashDrift), voices(json));
	}

	JsonObject toJson() {
		JsonObject json = new JsonObject();
		json.addProperty("volume", volume);
		json.addProperty("rain", rain);
		json.addProperty("wind", wind);
		json.addProperty("thunder", thunder);
		json.addProperty("water", water);
		json.addProperty("fire", fire);
		json.addProperty("lava", lava);
		json.addProperty("drips", drips);
		json.addProperty("strikes", strikes);
		json.addProperty("footsteps", footsteps);
		json.addProperty("birds", birds);
		json.addProperty("crickets", crickets);
		json.addProperty("frogs", frogs);
		json.addProperty("minecarts", minecarts);
		json.addProperty("doors", doors);
		json.addProperty("chests", chests);
		json.addProperty("redstone", redstone);
		json.addProperty("boats", boats);
		json.addProperty("sculk", sculk);
		json.addProperty("noteBlocks", noteBlocks);
		json.addProperty("caveAtmosphere", caveAtmosphere);
		json.addProperty("rarerSpecies", rarerSpecies);
		json.addProperty("underwater", underwater);
		json.addProperty("aeolian", aeolian);
		json.addProperty("biomeBeds", biomeBeds);
		json.addProperty("desertBeds", desertBeds);
		json.addProperty("mushroomHum", mushroomHum);
		json.addProperty("caveColors", caveColors);
		json.addProperty("deepDark", deepDark);
		json.addProperty("snowyHush", snowyHush);
		json.addProperty("mangroveDrone", mangroveDrone);
		json.addProperty("village", village);
		json.addProperty("settling", settling);
		json.addProperty("mob_brushes", mobBrushes);
		json.addProperty("peaceful_breathing", peacefulBreathing);
		json.addProperty("ash_drift", ashDrift);
		JsonObject mobs = new JsonObject();
		for (int i = 0; i < MOBS.size(); i++) mobs.addProperty(MOBS.get(i), voice(i));
		json.add("voices", mobs);
		return json;
	}

	/** The groups, as {@code Voices.GROUPS} (kept here so this class needs nothing of the game's). */
	static final java.util.List<String> MOBS = io.github.cumsauce.foley.audio.VoiceTable.GROUPS;

	private static long voices(JsonObject json) {
		if (!(json.get("voices") instanceof JsonObject mobs)) return DEFAULT.voices;
		long bits = 0;
		for (int i = 0; i < MOBS.size(); i++) {
			if (flag(mobs, MOBS.get(i), false)) bits |= 1L << i;
		}
		return bits;
	}

	private static boolean flag(JsonObject json, String key, boolean fallback) {
		return json.get(key) instanceof JsonPrimitive p && p.isBoolean() ? p.getAsBoolean() : fallback;
	}
}