package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.resources.Identifier;

/**
 * M24: the vanilla block step sounds Foley takes for its own footstep layer. Kept out of {@link Replacements} so its
 * id rule and verdict can be unit-tested without bootstrapping the game (the takeover sets there load {@code
 * SoundEvents}, which refuses to class-init in plain tests).
 */
final class Footsteps {
	private Footsteps() {}

	/**
	 * Whether {@code id} is one of the game's block step events Foley's {@link Layers} replaces: a block
	 * material's step sound, {@code block.<material>.step} (e.g. {@code block.grass.step}, {@code block.wool.step}).
	 *
	 * <p>The game plays a block's step only from its {@code SoundType}'s step sound, and {@link Materials} maps every
	 * sound type's five sounds — so every step event the game fires is one Layers layers a modal step under while the
	 * {@code footsteps} setting is on, and the suffix rule coincides exactly with what Foley replaces. Deliberately
	 * not matched: the same materials' {@code .fall}/{@code .hit} sounds (Layers gates those on the
	 * {@code strikes} setting, not {@code footsteps}; skipping them here would silence landings whenever strikes is
	 * off), {@code .break}/{@code .place} (M26's {@code blocks} takeover claims them), and entity steps (Decided #2
	 * keeps non-voice mob sounds vanilla).
	 */
	static boolean isStepEvent(Identifier id) {
		if (!id.getNamespace().equals("minecraft")) return false;
		String path = id.getPath();
		String prefix = "block.";
		String suffix = ".step";
		// "block.step" has nothing between prefix and suffix and names no material.
		return path.startsWith(prefix) && path.endsWith(suffix) && path.length() > prefix.length() + suffix.length();
	}

	/**
	 * The verdict on a block step event, with the footsteps setting's state supplied by the caller: skipped
	 * (counted) while Foley's layer plays, the game's where bypassed, declined by an override, or the setting off.
	 */
	static boolean replaceStep(SoundInstance sound, boolean wanted) {
		Identifier id = sound.getIdentifier();
		if (!isStepEvent(id)) return false;
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
		// Foley's contact layer is played here because the vanilla skip (the true return) prevents the
		// SoundEngineMixin else-branch from running it. In tests the game is not placed, so the layer is skipped.
		if (FoleyClient.placed() != null) Layers.layer(sound);
		FoleyClient.countReplaced();
		return true;
	}

	/** The verdict on {@code sound}: skipped for Foley's footsteps where the settings say so. */
	static boolean replaceStep(SoundInstance sound) {
		return replaceStep(sound, FoleyClient.wants(FoleyConfig::footsteps));
	}
}
