package io.github.cumsauce.foley.mixin;

import io.github.cumsauce.foley.audio.Voices;
import net.minecraft.client.resources.sounds.AbstractSoundInstance;
import net.minecraft.client.resources.sounds.Sound;
import net.minecraft.client.sounds.SoundManager;
import net.minecraft.client.sounds.WeighedSoundEvents;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Points a looping game sound Foley voices (a bee's buzz) at Foley's sound, once the game has resolved it. */
@Mixin(AbstractSoundInstance.class)
abstract class AbstractSoundInstanceMixin {
	@Shadow
	protected Sound sound;

	@Inject(method = "getOrResolve", at = @At("RETURN"))
	private void foley$redirect(SoundManager manager, CallbackInfoReturnable<WeighedSoundEvents> cir) {
		if (sound == null) return;
		Sound foley = Voices.redirect((AbstractSoundInstance) (Object) this);
		if (foley != null) sound = foley;
	}
}
