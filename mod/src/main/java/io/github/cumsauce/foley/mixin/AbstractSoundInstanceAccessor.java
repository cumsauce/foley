package io.github.cumsauce.foley.mixin;

import net.minecraft.client.resources.sounds.AbstractSoundInstance;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.gen.Accessor;

/**
 * A sound's own volume and pitch, before the game resolves it (its getters need the resolved sound), so a replacement
 * can keep them: a baby's higher voice, a quieter step.
 */
@Mixin(AbstractSoundInstance.class)
public interface AbstractSoundInstanceAccessor {
	@Accessor("volume")
	float foley$volume();

	@Accessor("pitch")
	float foley$pitch();
}
