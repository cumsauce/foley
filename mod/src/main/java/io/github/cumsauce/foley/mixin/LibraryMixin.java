package io.github.cumsauce.foley.mixin;

import com.mojang.blaze3d.audio.Library;
import net.minecraft.util.Mth;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Redirect;

/**
 * Gives streams a quarter of OpenAL's sources (64 of the usual 256) rather than the game's square root of them, at
 * most 8. Foley's beds (rain, wind, water, fire, lava, thunder) are streams, as are music and jukeboxes; all the beds
 * at once are 21, and with too few a bed that can't start is tried again every half second, each time with a warning
 * in the log. Sources are made only as sounds play, so the share costs nothing unused; the other three quarters stay
 * for the game's sounds.
 */
@Mixin(Library.class)
abstract class LibraryMixin {
	/** The game's {@code clamp((int) sqrt(sources), 2, 8)}: only the 8 used to be raised, so it was √256 = 16. */
	@Redirect(method = "init", at = @At(value = "INVOKE", target = "Lnet/minecraft/util/Mth;clamp(III)I", ordinal = 0))
	private int foley$moreStreams(int rootOfSources, int least, int most) {
		int sources = rootOfSources * rootOfSources;
		int streams = Mth.clamp(sources / 4, least, 64);
		io.github.cumsauce.foley.FoleyClient.LOGGER.info("Foley: {} of about {} sources for streams", streams, sources);
		return streams;
	}
}
