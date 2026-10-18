package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import io.github.cumsauce.foley.natives.NativeBridge;
import java.util.Set;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.resources.Identifier;

/**
 * M27: the vanilla explosion bang Foley takes for its own boom. Kept out of {@link Replacements} so its id rule and
 * verdict can be unit-tested without bootstrapping the game (the takeover sets there load {@code SoundEvents}, which
 * refuses to class-init in plain tests).
 */
final class Explosions {
	/** The game's bang paths, as literals: {@code SoundEvents} statics cannot class-init in plain tests. */
	private static final Set<String> BOOMS = Set.of("entity.generic.explode");

	private Explosions() {}

	/**
	 * Whether {@code id} is one of the game's explosion events Foley's boom replaces: {@code entity.generic.explode},
	 * the single vanilla bang every creeper, TNT blast and end crystal fires. Deliberately not matched yet:
	 * {@code entity.firework_rocket.large_blast} and the other firework variants (fireworks keep their character),
	 * which can widen later against the takeover table.
	 */
	static boolean isExplosionEvent(Identifier id) {
		return id.getNamespace().equals("minecraft") && BOOMS.contains(id.getPath());
	}

	/**
	 * The verdict on an explosion event, with the explosions setting's state supplied by the caller: skipped
	 * (counted) while Foley's bang plays, the game's where bypassed, declined by an override, or the setting off.
	 */
	static boolean replace(SoundInstance sound, boolean wanted) {
		Identifier id = sound.getIdentifier();
		if (!isExplosionEvent(id)) return false;
		// Bypass always lets the game's sound through, without consulting an override.
		if (FoleyClient.bypassed()) return false;
		// An override, if one matches, owns the event entirely — like every other takeover family.
		DoorHandling.Action action = DoorHandling.dispatch(id, sound.getX(), sound.getY(), sound.getZ());
		if (action == DoorHandling.Action.OVERRIDE_HANDLED) {
			FoleyClient.countReplaced();
			return true;
		}
		if (action == DoorHandling.Action.OVERRIDE_VANILLA) return false;
		if (!wanted) return false;
		// Foley's bang is played here because the vanilla skip (the true return) prevents the
		// SoundEngineMixin else-branch from running it. In tests the game is not placed, so it is skipped.
		if (FoleyClient.placed() != null) playBang(sound);
		FoleyClient.countReplaced();
		return true;
	}

	/** The verdict on {@code sound}: skipped for Foley's bang where the settings say so. */
	static boolean replace(SoundInstance sound) {
		return replace(sound, FoleyClient.wants(FoleyConfig::explosions));
	}

	/** Foley's explosion bang, rendered as a one-shot and played at the blast's position. */
	private static void playBang(SoundInstance sound) {
		var bridge = FoleyClient.bridge();
		if (bridge == null) return;
		// A deterministic-ish seed from the position: the same blast sounds the same on a replay.
		long seed = (long) (sound.getX() * 31 + sound.getY() * 17 + sound.getZ() * 13);
		short[] pcm = bridge.renderOneShot(NativeBridge.SHOT_BOOM, 2.0f, seed);
		// A BufferSound is LINEAR-attenuated, so the game's sound engine scales the bang with distance.
		Minecraft minecraft = Minecraft.getInstance();
		var boom = new BufferSound(Identifier.fromNamespaceAndPath("foley", "explosion"), 1.0f * FoleyClient.gain(),
				sound.getX(), sound.getY(), sound.getZ(), pcm);
		minecraft.getSoundManager().play(boom);
	}
}
