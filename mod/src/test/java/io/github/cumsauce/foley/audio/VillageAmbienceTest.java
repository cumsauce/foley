package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import io.github.cumsauce.foley.natives.NativeBridge;
import net.minecraft.core.BlockPos;
import org.junit.jupiter.api.Test;

/**
 * Unit tests for {@link VillageAmbience}. Full coverage of village detection needs a live
 * Minecraft ClientLevel, which the JUnit harness does not provide; this tests the things that
 * don't need one: the slot bookkeeping in {@link VillageBed}.
 */
class VillageAmbienceTest {
	@Test
	void new_village_ambience_has_played_zero() {
		// Construction with a null bridge is allowed: nothing actually fires without a tick.
		VillageAmbience ambience = new VillageAmbience(null);
		assertEquals(0, ambience.played());
		assertEquals(0, ambience.beds().size());
	}

	@Test
	void bed_bell_should_ring_rate_limits_per_bell() {
		VillageBed bed = new VillageBed(null);
		BlockPos a = new BlockPos(1, 2, 3);
		BlockPos b = new BlockPos(4, 5, 6);
		bed.see(a);
		bed.see(b);
		assertTrue(bed.bellShouldRing(a, 100, 50));
		assertFalse(bed.bellShouldRing(a, 120, 50));
		assertTrue(bed.bellShouldRing(b, 130, 50));
		bed.prune();
	}

	@Test
	void bed_prune_drops_slots_not_seen_this_scan() {
		VillageBed bed = new VillageBed(null);
		BlockPos a = new BlockPos(1, 2, 3);
		bed.bellShouldRing(a, 1, 0);
		assertEquals(1, bed.size());
		// Now see a different position; the unseen slot at (1,2,3) should be pruned.
		bed.see(new BlockPos(99, 99, 99));
		bed.prune();
		assertEquals(0, bed.size());
	}

	@Test
	void bed_murmur_rate_limits_per_bell() {
		VillageBed bed = new VillageBed(null);
		BlockPos a = new BlockPos(1, 2, 3);
		bed.see(a);
		assertTrue(bed.murmurShouldFire(a, 100, 5));
		assertFalse(bed.murmurShouldFire(a, 102, 5));
		assertTrue(bed.murmurShouldFire(a, 110, 5));
	}

	@Test
	void bed_animal_rate_limits_per_position() {
		VillageBed bed = new VillageBed(null);
		BlockPos a = new BlockPos(10, 10, 10);
		bed.see(a);
		assertTrue(bed.animalShouldFire(a, 1, 100));
		assertFalse(bed.animalShouldFire(a, 50, 100));
		assertTrue(bed.animalShouldFire(a, 200, 100));
	}
}
