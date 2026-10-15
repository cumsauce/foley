package io.github.cumsauce.foley.audio;

import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.mojang.serialization.JsonOps;
import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.api.MaterialDescriptor;
import io.github.cumsauce.foley.api.VoiceDescriptor;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Map;
import net.fabricmc.fabric.api.resource.ResourceManagerHelper;
import net.fabricmc.fabric.api.resource.SimpleSynchronousResourceReloadListener;
import net.minecraft.resources.Identifier;
import net.minecraft.server.packs.PackType;
import net.minecraft.server.packs.resources.Resource;
import net.minecraft.server.packs.resources.ResourceManager;
import net.minecraft.util.GsonHelper;

/**
 * The resource-pack form of addon sounds: every namespace's {@code assets/<ns>/foley/sounds.json}, read at each
 * client resource reload,
 *
 * <pre>{@code
 * { "big_drip": { "model": "drip_into_water", "radius_mm": 3.0, "variants": 4 } }
 * }</pre>
 *
 * <p>The entry's fields are the model's own ({@link io.github.cumsauce.foley.api.SoundCodecs}); a bad entry is logged
 * with its file and name and skipped, never taking the reload down.
 */
public final class AddonSoundFiles {
	private AddonSoundFiles() {}

	/** Starts listening for pack sounds. Client init. */
	public static void register() {
		ResourceManagerHelper.get(PackType.CLIENT_RESOURCES).registerReloadListener(new SimpleSynchronousResourceReloadListener() {
			@Override
			public Identifier getFabricId() {
				return Identifier.fromNamespaceAndPath("foley", "addon_sounds");
			}

			@Override
			public void onResourceManagerReload(ResourceManager manager) {
				readSounds(manager);
				readMaterials(manager);
				readVoices(manager);
			}

			/** Every namespace's {@code foley/voices.json}, as custom creatures and their call maps. */
			private void readVoices(ResourceManager manager) {
				for (Map.Entry<Identifier, Resource> file : manager.listResources("foley",
						id -> id.getPath().equals("foley/voices.json")).entrySet()) {
					String namespace = file.getKey().getNamespace();
					String where = namespace + ":foley/voices.json";
					try (BufferedReader reader = new BufferedReader(
							new InputStreamReader(file.getValue().open(), StandardCharsets.UTF_8))) {
						JsonObject root = GsonHelper.parse(reader);
						Map<String, CustomVoices.Entry> creatures = new HashMap<>();
						for (Map.Entry<String, JsonElement> entry : root.entrySet()) {
							try {
								JsonObject body = GsonHelper.convertToJsonObject(entry.getValue(),
										"\"" + entry.getKey() + "\"");
								VoiceDescriptor voice = VoiceDescriptor.CODEC.decoder()
										.parse(JsonOps.INSTANCE, GsonHelper.getAsJsonObject(body, "voice"))
										.getOrThrow();
								Map<String, String> calls = new HashMap<>();
								GsonHelper.getAsJsonObject(body, "calls").entrySet().forEach(call -> calls.put(
										call.getKey(), GsonHelper.convertToString(call.getValue(),
												"calls." + call.getKey())));
								creatures.put(entry.getKey(), new CustomVoices.Entry(voice, Map.copyOf(calls)));
							} catch (RuntimeException e) {
								FoleyClient.LOGGER.error("Foley: skipping {}'s \"{}\": {}", where, entry.getKey(),
										e.getMessage());
							}
						}
						CustomVoices.fromPacks(namespace, creatures);
					} catch (Exception e) {
						FoleyClient.LOGGER.error("Foley: could not read {}", where, e);
					}
				}
			}

			/** Every namespace's {@code foley/sounds.json}, as addon sound definitions. */
			private void readSounds(ResourceManager manager) {
				Map<Identifier, AddonSounds.Entry> parsed = new HashMap<>();
				for (Map.Entry<Identifier, Resource> file : manager.listResources("foley",
						id -> id.getPath().equals("foley/sounds.json")).entrySet()) {
					String namespace = file.getKey().getNamespace();
					String where = namespace + ":foley/sounds.json";
					try (BufferedReader reader = new BufferedReader(
							new InputStreamReader(file.getValue().open(), StandardCharsets.UTF_8))) {
						JsonObject root = GsonHelper.parse(reader);
						for (Map.Entry<String, JsonElement> entry : root.entrySet()) {
							String name = entry.getKey();
							try {
								if (!name.matches("[a-z0-9_]+"))
									throw new IllegalArgumentException("names are lowercase, digits and underscores");
								if (!entry.getValue().isJsonObject())
									throw new IllegalArgumentException("each sound is an object");
								parsed.put(Identifier.fromNamespaceAndPath(namespace, name),
										AddonSounds.parseEntry(entry.getValue().getAsJsonObject(), where + ", " + name));
							} catch (RuntimeException e) {
								FoleyClient.LOGGER.error("Foley: skipping {}'s \"{}\": {}", where, name, e.getMessage());
							}
						}
					} catch (Exception e) {
						FoleyClient.LOGGER.error("Foley: could not read {}", where, e);
					}
				}
				AddonSounds.fromPacks(parsed);
			}

			/** Every namespace's {@code foley/materials.json}, as addon materials and their block mappings. */
			private void readMaterials(ResourceManager manager) {
				for (Map.Entry<Identifier, Resource> file : manager.listResources("foley",
						id -> id.getPath().equals("foley/materials.json")).entrySet()) {
					String namespace = file.getKey().getNamespace();
					String where = namespace + ":foley/materials.json";
					try (BufferedReader reader = new BufferedReader(
							new InputStreamReader(file.getValue().open(), StandardCharsets.UTF_8))) {
						JsonObject root = GsonHelper.parse(reader);
						Map<String, MaterialDescriptor> materials = new HashMap<>();
						Map<String, String> blocks = new HashMap<>();
						Map<String, String> tags = new HashMap<>();
						if (root.has("materials")) {
							for (Map.Entry<String, JsonElement> entry : GsonHelper.getAsJsonObject(root, "materials")
									.entrySet()) {
								try {
									materials.put(entry.getKey(), MaterialDescriptor.CODEC.decoder()
											.parse(JsonOps.INSTANCE, entry.getValue()).getOrThrow());
								} catch (RuntimeException e) {
									FoleyClient.LOGGER.error("Foley: skipping {}'s material \"{}\": {}", where,
											entry.getKey(), e.getMessage());
								}
							}
						}
						if (root.has("blocks")) {
							GsonHelper.getAsJsonObject(root, "blocks").entrySet()
									.forEach(entry -> blocks.put(entry.getKey(), GsonHelper.convertToString(
											entry.getValue(), "blocks." + entry.getKey())));
						}
						if (root.has("tags")) {
							GsonHelper.getAsJsonObject(root, "tags").entrySet()
									.forEach(entry -> tags.put(entry.getKey(), GsonHelper.convertToString(
											entry.getValue(), "tags." + entry.getKey())));
						}
						CustomMaterials.fromPacks(namespace, materials, blocks, tags);
					} catch (Exception e) {
						FoleyClient.LOGGER.error("Foley: could not read {}", where, e);
					}
				}
			}
		});
	}
}
