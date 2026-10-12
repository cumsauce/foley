package io.github.cumsauce.foley.mixin;

import io.github.cumsauce.foley.audio.Replacements;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.client.sounds.SoundEngine;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Skips the game's sounds Foley replaces (see {@link Replacements}): they are replaced, not layered. */
@Mixin(SoundEngine.class)
abstract class SoundEngineMixin {
	@Inject(method = "play", at = @At("HEAD"), cancellable = true)
	private void foley$replace(SoundInstance sound, CallbackInfoReturnable<SoundEngine.PlayResult> cir) {
		if (Replacements.replace(sound)) cir.setReturnValue(SoundEngine.PlayResult.NOT_STARTED);
	}
}
