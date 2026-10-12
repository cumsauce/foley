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
 * @param voices     which mobs Foley voices (M6), a bit each in {@code Voices.MOBS}' order; none by default
 */
public record FoleyConfig(int volume, boolean rain, boolean wind, boolean thunder, boolean water, boolean fire,
		boolean lava, boolean drips, boolean strikes, boolean footsteps, boolean birds, boolean crickets,
		boolean frogs, int voices) {
	public static final FoleyConfig DEFAULT = new FoleyConfig(100, true, true, true, true, true, true, true, true, true,
			true, true, true, 0);

	/** Whether Foley voices the mob at {@code index} in {@code Voices.MOBS}. */
	public boolean voice(int index) {
		return index >= 0 && (voices & (1 << index)) != 0;
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
				flag(json, "frogs", d.frogs), voices(json));
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
		JsonObject mobs = new JsonObject();
		for (int i = 0; i < MOBS.size(); i++) mobs.addProperty(MOBS.get(i), voice(i));
		json.add("voices", mobs);
		return json;
	}

	/** The mobs, as {@code Voices.MOBS} (kept here so this class needs nothing of the game's). */
	static final java.util.List<String> MOBS = java.util.List.of("cow", "pig", "sheep", "chicken", "villager",
			"zombie", "wolf", "cat");

	private static int voices(JsonObject json) {
		if (!(json.get("voices") instanceof JsonObject mobs)) return DEFAULT.voices;
		int bits = 0;
		for (int i = 0; i < MOBS.size(); i++) {
			if (flag(mobs, MOBS.get(i), false)) bits |= 1 << i;
		}
		return bits;
	}

	private static boolean flag(JsonObject json, String key, boolean fallback) {
		return json.get(key) instanceof JsonPrimitive p && p.isBoolean() ? p.getAsBoolean() : fallback;
	}
}
