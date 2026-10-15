package io.github.cumsauce.foley.mixin;

import io.github.cumsauce.foley.audio.AddonSounds;
import io.github.cumsauce.foley.audio.GeneratedSounds;
import net.minecraft.client.resources.sounds.Sound;
import net.minecraft.client.sounds.SoundManager;
import net.minecraft.resources.Identifier;
import net.minecraft.server.packs.resources.ResourceProvider;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/**
 * Keeps Foley's generated sounds when the game checks sounds.json against files: they have none. An addon's, named
 * {@code <ns>:foley/<name>_<n>}, are kept the same way — the {@code foley/} path within a namespace is Foley's, by
 * convention.
 */
@Mixin(SoundManager.class)
abstract class SoundManagerMixin {
	@Inject(method = "validateSoundResource", at = @At("HEAD"), cancellable = true)
	private static void foley$generatedExist(Sound sound, Identifier event, ResourceProvider resources,
			CallbackInfoReturnable<Boolean> cir) {
		if (GeneratedSounds.isGenerated(sound.getLocation()) || AddonSounds.isGenerated(sound.getLocation()))
			cir.setReturnValue(true);
	}
}
