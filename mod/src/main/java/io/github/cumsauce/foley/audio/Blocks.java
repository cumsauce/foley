package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.resources.Identifier;

/**
 * M26: the vanilla block break, place and digging (mining-tap) sounds Foley replaces with its contact primitives.
 * Kept out of {@link Replacements} so its id rule and verdict can be unit-tested without bootstrapping the game.
 */
final class Blocks {
	private static final java.util.Set<String> MATERIALS = java.util.Set.of("stone", "wood", "gravel", "sand", "glass");
	private static final java.util.Set<String> CONTACTS = java.util.Set.of("hit", "place", "fall", "break");

	private Blocks() {}

	/** Whether {@code id} is a block contact event (hit/place/fall/break) on one of the narrow-start materials. */
	static boolean isBlockEvent(Identifier id) {
		if (!id.getNamespace().equals("minecraft")) return false;
		String path = id.getPath();
		if (!path.startsWith("block.")) return false;
		String rest = path.substring("block.".length());
		int dot = rest.lastIndexOf('.');
		if (dot < 0) return false;
		String material = rest.substring(0, dot);
		String contact = rest.substring(dot + 1);
		return MATERIALS.contains(material) && CONTACTS.contains(contact);
	}

	/** Standard family verdict: bypass -> vanilla; override verdict; then the blocks toggle. */
	static boolean replace(SoundInstance sound, boolean wanted) {
		Identifier id = sound.getIdentifier();
		if (!isBlockEvent(id)) return false;
		if (FoleyClient.bypassed()) return false;
		DoorHandling.Action action = DoorHandling.dispatch(id, sound.getX(), sound.getY(), sound.getZ());
		if (action == DoorHandling.Action.OVERRIDE_HANDLED) {
			FoleyClient.countReplaced();
			return true;
		}
		if (action == DoorHandling.Action.OVERRIDE_VANILLA) return false;
		if (!wanted) return false;
		// Foley's contact layer is played here because the vanilla skip prevents the SoundEngineMixin else-branch.
		if (FoleyClient.placed() != null) Layers.layer(sound);
		FoleyClient.countReplaced();
		return true;
	}

	static boolean replace(SoundInstance sound) {
		return replace(sound, FoleyClient.wants(FoleyConfig::blocks));
	}
}
