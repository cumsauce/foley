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

/** Explosions: Foley replaces the game's explosion bang with its own boom while the toggle is on. */
class ExplosionsReplacementTest {
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
		return new SimpleSoundInstance(SoundEvent.createVariableRangeEvent(event), SoundSource.HOSTILE, 1f, 1f,
				SoundInstance.createUnseededRandom(), 0, 0, 0);
	}

	@Test
	void theExplosionEventIsMatchedAndSkippedWhileExplosionsAreOn() {
		assertTrue(Explosions.isExplosionEvent(id("entity.generic.explode")));
		assertTrue(Explosions.replace(sound("entity.generic.explode"), true),
				"entity.generic.explode is skipped while explosions are on");
	}

	@Test
	void otherEventsAreNotMatched() {
		for (String other : new String[] {"entity.creeper.hurt", "block.stone.break", "entity.firework_rocket.shoot"}) {
			assertFalse(Explosions.isExplosionEvent(id(other)), other + " is not an explosion event");
			assertFalse(Explosions.replace(sound(other), true), other + " plays the game's sound");
		}
	}

	@Test
	void aForeignExplosionIdIsNotMatched() {
		Identifier foreign = Identifier.fromNamespaceAndPath("mymod", "entity.generic.explode");
		assertFalse(Explosions.isExplosionEvent(foreign));
		assertFalse(Explosions.replace(sound(foreign), true));
	}

	@Test
	void explosionsOffLeaveTheSoundToVanilla() {
		assertFalse(Explosions.replace(sound("entity.generic.explode"), false),
				"with explosions off the game's bang plays");
	}

	@Test
	void anOverrideOwnsTheExplosion() {
		DoorHandling.register(id -> id.getPath().equals("entity.generic.explode"), (event, x, y, z) -> true);
		assertTrue(Explosions.replace(sound("entity.generic.explode")));
	}

	@Test
	void aDecliningOverrideLeavesTheSoundToVanilla() {
		DoorHandling.register(id -> id.getPath().equals("entity.generic.explode"), (event, x, y, z) -> false);
		assertFalse(Explosions.replace(sound("entity.generic.explode")));
	}

	@Test
	void bypassIsNeutral() {
		FoleyClient.setBypassed(true);
		DoorHandling.register(id -> id.getPath().equals("entity.generic.explode"), (event, x, y, z) -> true);
		assertFalse(Explosions.replace(sound("entity.generic.explode")),
				"bypassed: no override is consulted and vanilla plays");
		FoleyClient.setBypassed(false);
	}
}
