package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.core.BlockPos;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.level.block.SoundType;
import net.minecraft.world.level.block.state.BlockState;

/**
 * Foley's strikes and breaks (M4), layered under the game's block sounds rather than replacing them, as decided: when
 * the game plays a block's hit (mining), place, fall (landing) or break sound, Foley plays its own modal one with it,
 * for the block's material. The material comes from the block where it's still there (the one hit or placed, or
 * landed on), else from the sound's {@link SoundType}.
 */
public final class Layers {
	/** The layer's volume for each contact: tap, place, land, break. */
	private static final float[] VOLUME = {0.5f, 0.6f, 0.8f, 0.9f};
	public static final String[] CONTACTS = {"tap", "place", "land", "break"};

	private Layers() {}

	/** Plays Foley's layer under {@code sound} if it's a block's. Called as the game is about to play it. */
	public static void layer(SoundInstance sound) {
		Identifier id = sound.getIdentifier();
		if (!id.getNamespace().equals("minecraft") || sound.isRelative()) return;
		SoundType type = Materials.soundTypeOf(id.toString());
		if (type == null) return;
		int contact;
		if (id.equals(type.getHitSound().location())) {
			contact = NativeBridge.CONTACT_TAP;
		} else if (id.equals(type.getPlaceSound().location())) {
			contact = NativeBridge.CONTACT_PLACE;
		} else if (id.equals(type.getFallSound().location())) {
			contact = NativeBridge.CONTACT_LAND;
		} else if (id.equals(type.getBreakSound().location())) {
			contact = NativeBridge.CONTACT_BREAK;
		} else {
			return;
		}
		Minecraft minecraft = Minecraft.getInstance();
		if (minecraft.level == null) return;
		int material = Materials.of(type);
		if (contact != NativeBridge.CONTACT_BREAK) {
			// Landing: the block under the feet the sound plays at; else the block itself.
			double below = contact == NativeBridge.CONTACT_LAND ? 0.2 : 0;
			BlockState state = minecraft.level.getBlockState(
					BlockPos.containing(sound.getX(), sound.getY() - below, sound.getZ()));
			if (!state.isAir()) material = Materials.of(state);
		}
		SoundEvent event = SoundEvent.createVariableRangeEvent(Identifier.fromNamespaceAndPath("foley",
				CONTACTS[contact] + "_" + Materials.NAMES[material]));
		minecraft.getSoundManager().play(new SimpleSoundInstance(event, SoundSource.BLOCKS, VOLUME[contact], 1f,
				SoundInstance.createUnseededRandom(), sound.getX(), sound.getY(), sound.getZ()));
		FoleyClient.countLayered();
	}
}
