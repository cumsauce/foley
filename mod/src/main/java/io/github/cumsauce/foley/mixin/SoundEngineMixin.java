package io.github.cumsauce.foley.mixin;

import io.github.cumsauce.foley.audio.Layers;
import io.github.cumsauce.foley.audio.Replacements;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.client.sounds.SoundEngine;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/**
 * Skips the game's sounds Foley replaces (see {@link Replacements}), and layers Foley's strikes and breaks under the
 * game's block sounds (see {@link Layers}).
 */
@Mixin(SoundEngine.class)
abstract class SoundEngineMixin {
	@Inject(method = "play", at = @At("HEAD"), cancellable = true)
	private void foley$replace(SoundInstance sound, CallbackInfoReturnable<SoundEngine.PlayResult> cir) {
		io.github.cumsauce.foley.audio.Life.heard(sound);
		if (Replacements.replace(sound)) {
			cir.setReturnValue(SoundEngine.PlayResult.NOT_STARTED);
		} else {
			Layers.layer(sound);
		}
	}
}
