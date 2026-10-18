package io.github.cumsauce.foley.audio;

import io.github.cumsauce.foley.FoleyClient;
import io.github.cumsauce.foley.config.FoleyConfig;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import net.minecraft.client.Minecraft;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.core.BlockPos;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundEvents;
import net.minecraft.sounds.SoundSource;
import net.minecraft.world.level.material.Fluids;

/**
 * The game's sounds Foley replaces (as decided: continuous environmental sounds are replaced, one-shots such as mobs
 * kept). Called as the game is about to play each sound:
 *
 * <ul>
 *   <li>its rain, while Foley's rain plays;</li>
 *   <li>its thunder (Foley strikes from the bolt itself, so whether Earshot held this back doesn't matter);</li>
 *   <li>its water, lava and fire ambience, from within reach of Foley's {@link Surroundings}; beyond, out to the
 *       game's 32 blocks, at most {@value #MOST_FAR} of each kind play at once, the rest skipped: a fire or a lava
 *       lake there played one per block, and ran the game out of sources;</li>
 *   <li>dripstone drips, swapped for a generated drip where the drop lands: into water if it lands in water or a
 *       cauldron, on stone otherwise, or a drop of lava.</li>
 *   <li>M21 active structures: door, chest, shulker and minecart events (the
 *       per-tick sounds Foley makes instead are played by {@link ActiveStructures});</li>
 *   <li>M24 footsteps: the block materials' step events ({@code block.<material>.step}), while the
 *       {@code footsteps} setting is on and Foley's {@link Layers} step plays instead.</li>
 * </ul>
 */
public final class Replacements {
	private static final Set<Identifier> RAIN = Set.of(SoundEvents.WEATHER_RAIN.location(),
			SoundEvents.WEATHER_RAIN_ABOVE.location());
	private static final Set<Identifier> THUNDER = Set.of(SoundEvents.LIGHTNING_BOLT_THUNDER.location(),
			SoundEvents.LIGHTNING_BOLT_IMPACT.location());
	private static final Set<Identifier> WATER = Set.of(SoundEvents.WATER_AMBIENT.location());
	private static final Set<Identifier> LAVA = Set.of(SoundEvents.LAVA_AMBIENT.location(),
			SoundEvents.LAVA_POP.location());
	private static final Set<Identifier> FIRE = Set.of(SoundEvents.FIRE_AMBIENT.location(),
			SoundEvents.CAMPFIRE_CRACKLE.location());
	private static final Identifier DRIP_WATER = SoundEvents.POINTED_DRIPSTONE_DRIP_WATER.location();
	private static final Identifier DRIP_WATER_INTO_CAULDRON =
			SoundEvents.POINTED_DRIPSTONE_DRIP_WATER_INTO_CAULDRON.location();
	private static final Set<Identifier> DRIP_LAVA = Set.of(SoundEvents.POINTED_DRIPSTONE_DRIP_LAVA.location(),
			SoundEvents.POINTED_DRIPSTONE_DRIP_LAVA_INTO_CAULDRON.location());
	/** M21: chest, shulker and minecart vanilla events, to skip so Foley can layer. */
	private static final Set<Identifier> CHEST = Set.of(SoundEvents.CHEST_OPEN.location(),
			SoundEvents.CHEST_CLOSE.location(), SoundEvents.ENDER_CHEST_OPEN.location(),
			SoundEvents.ENDER_CHEST_CLOSE.location());
	private static final Set<Identifier> SHULKER = Set.of(SoundEvents.SHULKER_BOX_OPEN.location(),
			SoundEvents.SHULKER_BOX_CLOSE.location());
	private static final Set<Identifier> MINECART = Set.of(SoundEvents.MINECART_INSIDE.location(),
			SoundEvents.MINECART_RIDING.location());
	/** At most this many of the game's water, lava or fire sounds play at once beyond Foley's reach, each. */
	static final int MOST_FAR = 6;
	/** Those playing now: water, lava, fire. Client thread. */
	private static final List<List<SoundInstance>> FAR = List.of(new ArrayList<>(), new ArrayList<>(),
			new ArrayList<>());
	public static final SoundEvent DRIP = event("drip");
	public static final SoundEvent DRIP_STONE = event("drip_stone");
	public static final SoundEvent DRIP_LAVA_EVENT = event("drip_lava");

	private Replacements() {}

	/** Whether to skip {@code sound}: true when Foley makes it instead. */
	public static boolean replace(SoundInstance sound) {
		Identifier id = sound.getIdentifier();
		if (Voices.replace(sound)) return true;
		if (RAIN.contains(id)) {
			// Bypass always lets the game's sound through, without consulting an override.
			if (FoleyClient.bypassed()) return false;
			Boolean owned = override(sound, id);
			if (owned != null) return owned;
			var rain = FoleyClient.rain();
			return rain != null && rain.replacing() && counted();
		}
		if (THUNDER.contains(id)) {
			if (FoleyClient.bypassed()) return false;
			Boolean owned = override(sound, id);
			if (owned != null) return owned;
			var thunder = FoleyClient.thunder();
			return thunder != null && thunder.replacing() && counted();
		}
		Minecraft minecraft = Minecraft.getInstance();
		// M23: note block strikes, taken for Foley's own instruments (the toggle is inside).
		if (NoteBlocks.replace(sound, minecraft)) return true;
		// M24: the game's block step events, taken for Foley's footsteps (the layer is played by Layers).
		if (Footsteps.replaceStep(sound)) return true;
		// M26: block break, place and digging on the narrow-start materials, taken for Foley's contact primitives.
		if (Blocks.replace(sound)) return true;
		if (FoleyClient.placed() == null || minecraft.level == null || minecraft.player == null) return false;
		boolean ambience = WATER.contains(id) || LAVA.contains(id) || FIRE.contains(id);
		if (ambience) {
			// Bypass always lets the game's sound through, without consulting an override.
			if (FoleyClient.bypassed()) return false;
			Boolean owned = override(sound, id);
			if (owned != null) return owned;
			boolean wanted = WATER.contains(id) ? FoleyClient.wants(FoleyConfig::water)
					: LAVA.contains(id) ? FoleyClient.wants(FoleyConfig::lava) : FoleyClient.wants(FoleyConfig::fire);
			if (!wanted) return false;
			if (Surroundings.within(minecraft.player.getEyePosition(), sound.getX(), sound.getY(), sound.getZ())) {
				return counted();
			}
			// Beyond reach: the game's own, but only so many at once; the rest blend in anyway.
			List<SoundInstance> far = FAR.get(WATER.contains(id) ? 0 : LAVA.contains(id) ? 1 : 2);
			far.removeIf(playing -> !minecraft.getSoundManager().isActive(playing));
			if (far.size() >= MOST_FAR) return counted();
			far.add(sound);
			return false;
		}
		SoundEvent drip = null;
		if (DRIP_LAVA.contains(id) || id.equals(DRIP_WATER_INTO_CAULDRON) || id.equals(DRIP_WATER)) {
			// A drip-family event: bypass always lets the game's sound through.
			if (FoleyClient.bypassed()) return false;
			Boolean owned = override(sound, id);
			if (owned != null) return owned;
		}
		if (!FoleyClient.wants(FoleyConfig::drips)) {
			// Still check M21 events before falling through: a toggled-off drips user may want doors on.
		} else if (DRIP_LAVA.contains(id)) {
			drip = DRIP_LAVA_EVENT;
		} else if (id.equals(DRIP_WATER_INTO_CAULDRON)) {
			drip = DRIP;
		} else if (id.equals(DRIP_WATER)) {
			BlockPos at = BlockPos.containing(sound.getX(), sound.getY() - 0.05, sound.getZ());
			drip = minecraft.level.getFluidState(at).getType().isSame(Fluids.WATER) ? DRIP : DRIP_STONE;
		}
		if (drip != null) {
			// The game's drips play at 0.3 to 1 of full volume.
			float volume = (0.3f + 0.7f * minecraft.level.getRandom().nextFloat()) * FoleyClient.gain();
			minecraft.getSoundManager().play(new SimpleSoundInstance(drip, SoundSource.BLOCKS, volume, 1f,
					SoundInstance.createUnseededRandom(), sound.getX(), sound.getY(), sound.getZ()));
			return counted();
		}
		// M21: doors, chests, shulkers, minecarts. Foley layers its own under the game's
		// event (which the dispatcher is about to skip) — the per-tick scan in
		// {@link ActiveStructures} plays the actual creak, the chest puff, the wheel impact.
		if (DoorHandling.isDoorEvent(id)) {
			boolean iron = DoorHandling.isIronDoorEvent(id);
			// Bypass always lets the game's sound through, without consulting an override.
			if (FoleyClient.bypassed()) return false;
			// An override, if one matches, owns the event entirely: no built-in tap when it handles
			// the event, the game's sound when it declines it.
			Boolean owned = override(sound, id);
			if (owned != null) return owned;
			if (FoleyClient.activeStructures() != null && FoleyClient.wants(FoleyConfig::doors)) {
				FoleyClient.activeStructures().onDoorSkipped(minecraft, sound, iron);
				return counted();
			}
			return false;
		}
		if (FoleyClient.activeStructures() != null) {
			if (CHEST.contains(id)) {
				// Bypass always lets the game's sound through, without consulting an override.
				if (FoleyClient.bypassed()) return false;
				Boolean owned = override(sound, id);
				if (owned != null) return owned;
				if (FoleyClient.wants(FoleyConfig::chests)) {
					FoleyClient.activeStructures().onChestSkipped(minecraft, sound, false);
					return counted();
				}
				return false;
			}
			if (SHULKER.contains(id)) {
				if (FoleyClient.bypassed()) return false;
				Boolean owned = override(sound, id);
				if (owned != null) return owned;
				if (FoleyClient.wants(FoleyConfig::chests)) {
					FoleyClient.activeStructures().onChestSkipped(minecraft, sound, true);
					return counted();
				}
				return false;
			}
			if (MINECART.contains(id)) {
				if (FoleyClient.bypassed()) return false;
				Boolean owned = override(sound, id);
				if (owned != null) return owned;
				// The game plays a constant drone while riding; we just let Foley's per-tick
				// rush bed take over and skip the game's own.
				return FoleyClient.wants(FoleyConfig::minecarts) && counted();
			}
		}
		return false;
	}

	/**
	 * An override's verdict on {@code id}: TRUE when it handles the event (counted as replaced), FALSE when it
	 * declines it (the game's sound plays), {@code null} when none matched and the built-in logic should run.
	 */
	private static Boolean override(SoundInstance sound, Identifier id) {
		DoorHandling.Action action = DoorHandling.dispatch(id, sound.getX(), sound.getY(), sound.getZ());
		if (action == DoorHandling.Action.OVERRIDE_HANDLED) {
			counted();
			return Boolean.TRUE;
		}
		if (action == DoorHandling.Action.OVERRIDE_VANILLA) return Boolean.FALSE;
		return null;
	}

	private static boolean counted() {
		FoleyClient.countReplaced();
		return true;
	}

	private static SoundEvent event(String name) {
		return Sounds.event(name);
	}
}
