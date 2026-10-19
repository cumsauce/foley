package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

/**
 * What startles the birds. Foley's own sounds must never do it: the thunder bed is registered as
 * {@code foley:thunder} and re-plays itself for as long as a storm rumbles, so treating it as a
 * startle keeps {@code Life}'s silence window refreshed for the whole storm — birds then never sing
 * at all, which is what the client gametest caught at the dawn chorus.
 */
class LifeStartleTest {
	@Test
	void theGamesThunderStartlesTheBirds() {
		assertTrue(Life.startles("minecraft:entity.lightning_bolt.thunder"));
	}

	@Test
	void anExplosionStartlesTheBirds() {
		assertTrue(Life.startles("minecraft:entity.generic.explode"));
		assertTrue(Life.startles("minecraft:entity.firework_rocket.blast"));
	}

	@Test
	void foleysOwnThunderBedNeverStartlesTheBirds() {
		// The bed Foley plays for a storm is foley:thunder, and it re-plays until the storm passes.
		assertFalse(Life.startles("foley:thunder"));
	}

	@Test
	void noOtherFoleySoundStartlesTheBirds() {
		assertFalse(Life.startles("foley:rain"));
		assertFalse(Life.startles("foley:wind"));
	}

	@Test
	void anOrdinarySoundNeverStartlesTheBirds() {
		assertFalse(Life.startles("minecraft:block.grass.break"));
		assertFalse(Life.startles("minecraft:entity.bird_5"));
	}
}
