package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertTrue;

import net.minecraft.world.entity.animal.cow.Cow;
import net.minecraft.world.entity.animal.sheep.Sheep;
import org.junit.jupiter.api.Test;

/** Peaceful-mob breathing (M22): the kinds of mob the scan looks for. */
class PeacefulBreathingTest {
	@Test
	void theMobsAreCowsAndSheep() {
		Class<?>[] kinds = PeacefulBreathing.KINDS;
		assertTrue(java.util.Arrays.asList(kinds).contains(Cow.class), "cow");
		assertTrue(java.util.Arrays.asList(kinds).contains(Sheep.class), "sheep");
		assertTrue(kinds.length == 2, "no extras: " + kinds.length);
	}

	@Test
	void theReachIsTheSameAsTheZombieBreath() {
		// The peaceful mob is heard at the same range as a zombie: the listener
		// doesn't have to learn a new distance for a quiet cow.
		assertTrue(PeacefulBreathing.REACH == 10);
	}
}