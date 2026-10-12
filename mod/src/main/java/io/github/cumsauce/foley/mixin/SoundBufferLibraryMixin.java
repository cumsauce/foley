package io.github.cumsauce.foley.mixin;

import com.mojang.blaze3d.audio.SoundBuffer;
import io.github.cumsauce.foley.audio.GeneratedSounds;
import java.util.concurrent.CompletableFuture;
import net.minecraft.client.sounds.SoundBufferLibrary;
import net.minecraft.resources.Identifier;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/** Answers for Foley's generated one-shots, which have no file, with buffers rendered from their models. */
@Mixin(SoundBufferLibrary.class)
abstract class SoundBufferLibraryMixin {
	@Inject(method = "getCompleteBuffer", at = @At("HEAD"), cancellable = true)
	private void foley$generated(Identifier path, CallbackInfoReturnable<CompletableFuture<SoundBuffer>> cir) {
		if (!path.getNamespace().equals("foley")) return;
		SoundBuffer buffer = GeneratedSounds.render(path);
		if (buffer != null) cir.setReturnValue(CompletableFuture.completedFuture(buffer));
	}
}
