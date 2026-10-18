package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.github.cumsauce.foley.FoleyClient;
import java.util.List;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

/**
 * M24: the game's block step events are Foley's alone while the footsteps setting is on. The matcher is exactly the
 * block materials' step sounds — {@code block.<material>.step}, e.g. {@code block.grass.step} — and nothing else: the
 * same materials' land, tap, break and place events stay the game's (M26's takeover), and so do every entity's steps
 * (Decided #2). Ids are written out here as the other families' tests do, so no registry or client is needed.
 */
class FootstepsReplacementTest {
	/** The materials' step ids from the game's sound types, one per material family. */
	private static final List<String> STEPS = List.of("block.grass.step", "block.stone.step", "block.gravel.step",
			"block.wood.step", "block.metal.step", "block.sand.step", "block.snow.step", "block.glass.step",
			"block.netherrack.step", "block.moss.step", "block.wool.step", "block.slime.step", "block.amethyst.step",
			"block.bamboo_sapling.step", "block.crop.step", "block.roots.step");

	@AfterEach
	void cleanUp() {
		DoorHandling.clearOverrides();
		FoleyClient.setBypassed(false);
	}

	@Test
	void aStepSoundIsSkippedWhenFootstepsAreOn() {
		for (String path : STEPS) {
			assertTrue(Footsteps.isStepEvent(id(path)), path + " is a step event");
			assertTrue(Footsteps.replaceStep(sound(path)), path + " is skipped while footsteps are on");
		}
	}

	@Test
	void onlyTheStepOfEachMaterialIsMatchedNotItsSiblings() {
		for (String material : List.of("grass", "stone", "gravel", "wood", "metal", "sand", "snow", "glass")) {
			for (String event : List.of("fall", "hit", "break", "place")) {
				String path = "block." + material + "." + event;
				assertFalse(Footsteps.isStepEvent(id(path)),
						path + " is not a step event: land, dig, break and place stay vanilla");
				assertFalse(Footsteps.replaceStep(sound(path)), path + " falls through to the game");
			}
		}
	}

	@Test
	void anUnrelatedBlockSoundIsNotSkipped() {
		for (String path : List.of("block.stone.break", "block.stone.place", "block.stone.hit", "block.grass.fall",
				"block.note_block.harp", "block.chest.open", "block.pointed_dripstone.drip_water", "weather.rain",
				"entity.minecart.riding", "block.step")) {
			assertFalse(Footsteps.replaceStep(sound(path)), path + " falls through to the game");
		}
	}

	@Test
	void anEntityStepSoundIsNotSkipped() {
		// Decided #2: mob footsteps stay vanilla; only block materials' step events are Foley's.
		assertFalse(Footsteps.isStepEvent(id("entity.horse.step")));
		assertFalse(Footsteps.isStepEvent(id("entity.generic.footstep")));
		assertFalse(Footsteps.replaceStep(sound("entity.horse.step")));
	}

	@Test
	void aForeignStepIdIsNotSkipped() {
		// A mod's own sound type keeps its step sounds; Foley only takes the game's materials.
		assertFalse(Footsteps.isStepEvent(Identifier.fromNamespaceAndPath("mymod", "block.grass.step")));
		assertFalse(Footsteps.replaceStep(sound(Identifier.fromNamespaceAndPath("mymod", "block.grass.step"))));
	}

	@Test
	void footstepsOffLeaveTheStepToVanilla() {
		assertFalse(Footsteps.replaceStep(sound("block.grass.step"), false),
				"with the setting off, and no override matching, the game's step plays");
		assertTrue(Footsteps.replaceStep(sound("block.grass.step"), true));
	}

	@Test
	void anOverrideOwnsAStepSound() {
		DoorHandling.register(id -> id.getPath().equals("block.grass.step"), (event, x, y, z) -> true);
		assertTrue(Footsteps.replaceStep(sound("block.grass.step")),
				"an override's handled verdict wins over the toggle");
	}

	@Test
	void aDecliningOverrideLeavesTheStepToVanilla() {
		DoorHandling.register(id -> id.getPath().equals("block.grass.step"), (event, x, y, z) -> false);
		assertFalse(Footsteps.replaceStep(sound("block.grass.step")));
	}

	@Test
	void bypassIsNeutral() {
		FoleyClient.setBypassed(true);
		assertFalse(Footsteps.replaceStep(sound("block.grass.step")),
				"bypassed: the game's step plays although footsteps are on");
		DoorHandling.register(id -> id.getPath().equals("block.grass.step"), (event, x, y, z) -> true);
		assertFalse(Footsteps.replaceStep(sound("block.grass.step")), "bypassed: no override is consulted either");
	}

	private static Identifier id(String path) {
		return Identifier.fromNamespaceAndPath("minecraft", path);
	}

	/** The game's sound at that id, played at the origin — the fakes every family's tests use. */
	private static SoundInstance sound(String path) {
		return sound(id(path));
	}

	/** The same, at an explicit id — for ids outside the {@code minecraft} namespace. */
	private static SoundInstance sound(Identifier event) {
		return new SimpleSoundInstance(SoundEvent.createVariableRangeEvent(event), SoundSource.BLOCKS, 1f, 1f,
				SoundInstance.createUnseededRandom(), 0, 0, 0);
	}
}
