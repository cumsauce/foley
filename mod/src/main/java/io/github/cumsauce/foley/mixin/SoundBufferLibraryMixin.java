package io.github.cumsauce.foley.mixin;

import com.mojang.blaze3d.audio.SoundBuffer;
import io.github.cumsauce.foley.audio.AddonSounds;
import io.github.cumsauce.foley.audio.GeneratedSounds;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import net.minecraft.client.sounds.SoundBufferLibrary;
import net.minecraft.util.Util;
import net.minecraft.resources.Identifier;
import org.spongepowered.asm.mixin.Final;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

/**
 * Answers for Foley's generated one-shots — its own and other mods' — which have no file, with buffers rendered from
 * their models, kept in the library's own cache like a loaded file: each variant is made once, and its OpenAL buffer
 * freed when the game clears the cache (on a resource reload). Without it, every play made it again and leaked a
 * buffer.
 */
@Mixin(SoundBufferLibrary.class)
abstract class SoundBufferLibraryMixin {
	@Shadow
	@Final
	private Map<Identifier, CompletableFuture<SoundBuffer>> cache;

	@Inject(method = "getCompleteBuffer", at = @At("HEAD"), cancellable = true)
	private void foley$generated(Identifier path, CallbackInfoReturnable<CompletableFuture<SoundBuffer>> cir) {
		if (!GeneratedSounds.isGeneratedPath(path) && !AddonSounds.isGeneratedPath(path)) return;
		// Rendered off the thread that asked, as the game loads files: a shatter takes a few milliseconds.
		cir.setReturnValue(cache.computeIfAbsent(path, p -> CompletableFuture.supplyAsync(() -> {
			SoundBuffer buffer = GeneratedSounds.render(p);
			if (buffer == null) buffer = AddonSounds.render(p);
			if (buffer == null) throw new IllegalArgumentException("no generated sound " + p);
			return buffer;
		}, Util.backgroundExecutor())));
	}
}
