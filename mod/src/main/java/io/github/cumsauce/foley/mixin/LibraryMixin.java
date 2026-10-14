package io.github.cumsauce.foley.mixin;

import com.mojang.blaze3d.audio.Library;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.Constant;
import org.spongepowered.asm.mixin.injection.ModifyConstant;

/**
 * Lets the game stream up to 32 sounds at once rather than 8. Its streaming pool is the square root of the sources
 * OpenAL gives (256 by default), at most 8, shared by music and jukeboxes; Foley's beds (rain, wind, water, fire,
 * lava, thunder) are streams too, and in a storm by a waterfall with music on, they ran out: a bed that can't start is
 * tried again every half second, each time with a warning in the log. All of Foley's beds at once are 21; the other
 * 220 or so sources stay for the game's sounds.
 */
@Mixin(Library.class)
abstract class LibraryMixin {
	@ModifyConstant(method = "init", constant = @Constant(intValue = 8, ordinal = 0))
	private int foley$moreStreams(int most) {
		return 32;
	}
}
