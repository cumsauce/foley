package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.api.FoleyBeds;
import java.util.HashMap;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.resources.Identifier;
import org.jspecify.annotations.Nullable;

/**
 * Other mods' beds (F2): Foley's own streaming models — rain, wind, water, fire, lava, thunder — under ids of the
 * addon's choosing, told their scenes as the addon's world changes. At most {@link FoleyBeds#MOST} at once: the
 * game's 8 streams are nearly spent between the game's music and Foley's own beds.
 */
public final class AddonBeds {
	/** The scene lengths each kind's native model takes (see {@code foley.h}). */
	private static final int[] PARAMS = {10, 3, 3, 2, 2, 3};

	private static final Map<Identifier, BedSound<AddonScene>> RUNNING = new HashMap<>();
	private static int started;

	private AddonBeds() {}

	/** A scene of plain floats, as the API's beds take. */
	record AddonScene(float[] params) implements Scene {}

	/** The beds, as one addon sees it: every refusal names its addon. */
	static FoleyBeds api(String origin) {
		return new FoleyBeds() {
			@Override
			public boolean start(Identifier id, BedKind kind, float[] scene) {
				// The namespace rule binds addons in their entrypoint, where each is named; beds also start at
				// runtime through the facade, where an addon's id simply is its own.
				if (!origin.isEmpty() && !origin.equals(id.getNamespace()))
					throw new IllegalArgumentException(origin + " may not start " + id + ": a bed's namespace is its owner's");
				if (kind == null) throw new IllegalArgumentException(id + " needs a bed kind");
				if (scene == null || scene.length != PARAMS[kind.ordinal()])
					throw new IllegalArgumentException(id + " takes " + PARAMS[kind.ordinal()]
							+ " floats for a " + kind + " bed, not " + (scene == null ? "none" : scene.length));
				if (RUNNING.containsKey(id)) return false;
				if (RUNNING.size() >= FoleyBeds.MOST) return false;
				Minecraft minecraft = Minecraft.getInstance();
				if (minecraft.level == null || minecraft.player == null || FoleyClient.bridge() == null) return false;
				BedSound<AddonScene> sound = BedSound.addon(
						"addon/" + id.getNamespace() + "_" + id.getPath() + "_" + started++,
						FoleyClient.bridge(), kind.ordinal(), scene);
				sound.place(minecraft.player.position());
				minecraft.getSoundManager().play(sound);
				RUNNING.put(id, sound);
				return true;
			}

			@Override
			public void scene(Identifier id, float[] scene) {
				BedSound<AddonScene> sound = RUNNING.get(id);
				if (sound == null) return;
				if (scene == null || scene.length != PARAMS[kindOf(id)]) return;
				sound.setScene(new AddonScene(scene));
			}

			@Override
			public void place(Identifier id, double x, double y, double z) {
				BedSound<AddonScene> sound = RUNNING.get(id);
				if (sound != null) sound.place(new net.minecraft.world.phys.Vec3(x, y, z));
			}

			@Override
			public void weight(Identifier id, float weight) {
				BedSound<AddonScene> sound = RUNNING.get(id);
				if (sound != null) sound.weigh(java.lang.Math.max(0f, java.lang.Math.min(1f, weight)));
			}

			@Override
			public boolean playing(Identifier id) {
				return RUNNING.containsKey(id);
			}

			@Override
			public void stop(Identifier id) {
				BedSound<AddonScene> sound = RUNNING.remove(id);
				if (sound != null) sound.retire();
			}
		};
	}

	/** The kind the bed at {@code id} was started with, from its sound's own kind. */
	private static int kindOf(Identifier id) {
		return RUNNING.get(id).nativeKind();
	}

	/** How many samples {@code id}'s bed has rendered; 0 if it isn't running. For the client gametest. */
	public static long rendered(Identifier id) {
		BedSound<AddonScene> sound = RUNNING.get(id);
		return sound == null ? 0 : sound.rendered();
	}
}
