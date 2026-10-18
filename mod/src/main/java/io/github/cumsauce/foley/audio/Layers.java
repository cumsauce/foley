package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
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
 * Foley's strikes, breaks (M4) and footsteps (M5), layered under the game's block sounds rather than replacing them, as
 * decided: when the game plays a block's hit (mining), place, fall (landing), break or step sound, Foley plays its own modal one with it,
 * for the block's material. The material comes from the block where it's still there (the one hit or placed, or
 * landed on), else from the sound's {@link SoundType}.
 */
public final class Layers {
	/** The layer's volume for each contact: tap, place, land, break, step. */
	private static final float[] VOLUME = {0.5f, 0.6f, 0.8f, 0.9f, 0.18f};
	public static final String[] CONTACTS = {"tap", "place", "land", "break", "step"};

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
		} else if (id.equals(type.getStepSound().location())) {
			contact = NativeBridge.CONTACT_STEP;
		} else {
			return;
		}
		boolean layerWanted = (contact == NativeBridge.CONTACT_STEP)
				? FoleyClient.config().footsteps()
				: (FoleyClient.config().strikes() || FoleyClient.config().blocks());
		if (!layerWanted) {
			return;
		}
		Minecraft minecraft = Minecraft.getInstance();
		if (minecraft.level == null || minecraft.player == null) return;
		// Far layers aren't heard under the game's sound anyway; near ones within the cap.
		if (minecraft.player.distanceToSqr(sound.getX(), sound.getY(), sound.getZ()) > 16 * 16 || !FoleyClient.oneShot()) {
			return;
		}
		// The custom material, if any: the block itself (exact, then a tag) where it can be asked, else its sound
		// group — the only route for a break, whose block is already gone.
		Identifier custom = null;
		if (contact != NativeBridge.CONTACT_BREAK) {
			// Landing and stepping: the block under the feet the sound plays at; else the block itself.
			double below = contact == NativeBridge.CONTACT_LAND || contact == NativeBridge.CONTACT_STEP ? 0.2 : 0;
			BlockState state = minecraft.level.getBlockState(
					BlockPos.containing(sound.getX(), sound.getY() - below, sound.getZ()));
			if (!state.isAir()) custom = CustomMaterials.of(state);
		}
		if (custom == null) custom = CustomMaterials.of(type);
		if (custom != null) {
			// A custom layer has no sounds.json entry: it resolves itself to one of its eight variants.
			String name = CustomMaterials.layerName(custom, contact);
			if (name == null) return;
			minecraft.getSoundManager().play(AddonSounds.instance(Sounds.event(name).location(),
					Identifier.fromNamespaceAndPath("foley", "generated/" + name + "_"
							+ SoundInstance.createUnseededRandom().nextInt(8)),
					SoundSource.BLOCKS, VOLUME[contact] * FoleyClient.gain(), 1f, sound.getX(), sound.getY(),
					sound.getZ()));
		} else {
			int material = Materials.of(type);
			if (contact != NativeBridge.CONTACT_BREAK) {
				double below = contact == NativeBridge.CONTACT_LAND || contact == NativeBridge.CONTACT_STEP ? 0.2 : 0;
				BlockState state = minecraft.level.getBlockState(
						BlockPos.containing(sound.getX(), sound.getY() - below, sound.getZ()));
				if (!state.isAir()) material = Materials.of(state);
			}
			SoundEvent event = Sounds.event(CONTACTS[contact] + "_" + Materials.NAMES[material]);
			minecraft.getSoundManager().play(new SimpleSoundInstance(event, SoundSource.BLOCKS,
					VOLUME[contact] * FoleyClient.gain(), 1f, SoundInstance.createUnseededRandom(), sound.getX(),
					sound.getY(), sound.getZ()));
			// M22: a delayed settle tick behind footsteps and block placement on
			// gravel, sand or snow — granular shifts and a snow brush.
			if (contact != NativeBridge.CONTACT_BREAK && FoleyClient.settling() != null) {
				FoleyClient.settling().settle(minecraft, sound.getX(), sound.getY(), sound.getZ(), material);
			}
		}
		FoleyClient.countLayered();
	}
}
