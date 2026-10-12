package io.github.cumsauce.foley.mixin;

import io.github.cumsauce.foley.FoleyClient;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.client.sounds.SoundEngine;
import net.minecraft.sounds.SoundEvents;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Skips the game's own rain sounds while Foley makes the rain: they are replaced, not layered. */
@Mixin(SoundEngine.class)
abstract class SoundEngineMixin {
	@Inject(method = "play", at = @At("HEAD"), cancellable = true)
	private void foley$replaceRain(SoundInstance sound, CallbackInfoReturnable<SoundEngine.PlayResult> cir) {
		var rain = FoleyClient.rain();
		if (rain == null || !rain.replacing()) return;
		var id = sound.getIdentifier();
		if (id.equals(SoundEvents.WEATHER_RAIN.location()) || id.equals(SoundEvents.WEATHER_RAIN_ABOVE.location())) {
			FoleyClient.countReplaced();
			cir.setReturnValue(SoundEngine.PlayResult.NOT_STARTED);
		}
	}
}
