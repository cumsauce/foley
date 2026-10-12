package io.github.cumsauce.foley.mixin;

import io.github.cumsauce.foley.audio.GeneratedSounds;
import net.minecraft.client.resources.sounds.Sound;
import net.minecraft.client.sounds.SoundManager;
import net.minecraft.resources.Identifier;
import net.minecraft.server.packs.resources.ResourceProvider;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Keeps Foley's generated sounds when the game checks sounds.json against files: they have none. */
@Mixin(SoundManager.class)
abstract class SoundManagerMixin {
	@Inject(method = "validateSoundResource", at = @At("HEAD"), cancellable = true)
	private static void foley$generatedExist(Sound sound, Identifier event, ResourceProvider resources,
			CallbackInfoReturnable<Boolean> cir) {
		if (GeneratedSounds.isGenerated(sound.getLocation())) cir.setReturnValue(true);
	}
}
