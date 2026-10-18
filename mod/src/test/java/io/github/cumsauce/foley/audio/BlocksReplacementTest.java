package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.github.cumsauce.foley.FoleyClient;
import net.minecraft.client.resources.sounds.SimpleSoundInstance;
import net.minecraft.client.resources.sounds.SoundInstance;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.sounds.SoundSource;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

/** Block break, place and digging: Foley replaces the game's contact sounds on the narrow-start materials. */
class BlocksReplacementTest {
	@AfterEach
	void cleanUp() {
		DoorHandling.clearOverrides();
		FoleyClient.setBypassed(false);
	}

	private static Identifier id(String path) {
		return Identifier.fromNamespaceAndPath("minecraft", path);
	}

	private static SoundInstance sound(String path) {
		return sound(id(path));
	}

	private static SoundInstance sound(Identifier event) {
		return new SimpleSoundInstance(SoundEvent.createVariableRangeEvent(event), SoundSource.BLOCKS, 1f, 1f,
				SoundInstance.createUnseededRandom(), 0, 0, 0);
	}

	@Test
	void aBlockSoundIsSkippedWhenBlocksAreOn() {
		for (String material : new String[] {"stone", "wood", "gravel", "sand", "glass"}) {
			for (String contact : new String[] {"hit", "place", "fall", "break"}) {
				assertTrue(Blocks.isBlockEvent(id("block." + material + "." + contact)),
						material + "." + contact + " is a block event");
				assertTrue(Blocks.replace(sound("block." + material + "." + contact)),
						material + "." + contact + " is skipped while blocks are on");
			}
		}
	}

	@Test
	void onlyBlockContactsOnNarrowMaterialsAreMatched() {
		// step is M24's, and other suffixes/materials are not in the narrow start.
		for (String contact : new String[] {"step", "ambient", "dig"}) {
			assertFalse(Blocks.isBlockEvent(id("block.stone." + contact)),
					"stone." + contact + " is not a blocks event");
		}
		for (String material : new String[] {"deepslate", "netherrack", "dirt", "grass"}) {
			assertFalse(Blocks.isBlockEvent(id("block." + material + ".break")),
					material + ".break is not in the narrow start");
		}
	}

	@Test
	void anUnrelatedBlockSoundIsNotSkipped() {
		assertFalse(Blocks.isBlockEvent(id("block.stone.ambient")));
		assertFalse(Blocks.replace(sound("block.stone.ambient")));
	}

	@Test
	void anEntitySoundIsNotSkipped() {
		assertFalse(Blocks.isBlockEvent(id("entity.player.hurt")));
		assertFalse(Blocks.replace(sound("entity.player.hurt")));
	}

	@Test
	void aForeignBlockIdIsNotSkipped() {
		assertFalse(Blocks.isBlockEvent(Identifier.fromNamespaceAndPath("mymod", "block.stone.break")));
		assertFalse(Blocks.replace(sound(Identifier.fromNamespaceAndPath("mymod", "block.stone.break"))));
	}

	@Test
	void blocksOffLeaveTheSoundToVanilla() {
		assertFalse(Blocks.replace(sound("block.stone.break"), false),
				"with blocks off the game's sound plays");
	}

	@Test
	void anOverrideOwnsABlockSound() {
		DoorHandling.register(id -> id.getPath().equals("block.stone.break"), (event, x, y, z) -> true);
		assertTrue(Blocks.replace(sound("block.stone.break")));
	}

	@Test
	void aDecliningOverrideLeavesTheSoundToVanilla() {
		DoorHandling.register(id -> id.getPath().equals("block.stone.break"), (event, x, y, z) -> false);
		assertFalse(Blocks.replace(sound("block.stone.break")));
	}

	@Test
	void bypassIsNeutral() {
		FoleyClient.setBypassed(true);
		DoorHandling.register(id -> id.getPath().equals("block.stone.break"), (event, x, y, z) -> true);
		assertFalse(Blocks.replace(sound("block.stone.break")),
				"bypassed: no override is consulted and vanilla plays");
		FoleyClient.setBypassed(false);
	}
}
