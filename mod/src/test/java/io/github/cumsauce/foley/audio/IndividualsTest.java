package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.HashSet;
import java.util.Random;
import java.util.Set;
import java.util.UUID;
import org.junit.jupiter.api.Test;

class IndividualsTest {
	@Test
	void eachMobKeepsItsOwnVoiceAndTheyDiffer() {
		UUID cow = UUID.fromString("5f3c2a1e-9b7d-4c8e-a1f2-3d4e5f6a7b8c");
		assertEquals(Individuals.voice(cow), Individuals.voice(UUID.fromString(cow.toString())));
		float low = 2, high = 0;
		Random random = new Random(1);
		for (int i = 0; i < 10_000; i++) {
			float voice = Individuals.voice(new UUID(random.nextLong(), random.nextLong()));
			assertTrue(voice >= 0.93f && voice <= 1.07f, "voice " + voice);
			low = Math.min(low, voice);
			high = Math.max(high, voice);
		}
		// They spread over most of the range.
		assertTrue(high - low > 0.12f, low + " to " + high);
	}

	@Test
	void aMobMostlyCallsInItsOwnFewWays() {
		Random random = new Random(2);
		UUID cow = new UUID(random.nextLong(), random.nextLong());
		int[] counts = new int[12];
		for (int i = 0; i < 10_000; i++) {
			int n = Individuals.rendition(cow, random.nextFloat(), random.nextInt(12), 12);
			assertTrue(n >= 0 && n < 12);
			counts[n]++;
		}
		int[] sorted = counts.clone();
		java.util.Arrays.sort(sorted);
		// Its three habits take about three quarters of its calls; it still uses the others now and then.
		int habits = sorted[11] + sorted[10] + sorted[9];
		assertTrue(habits > 7_000 && habits < 8_200, "habits " + habits);
		assertTrue(sorted[0] > 100, "the rest " + sorted[0]);
		// Different mobs have different habits.
		Set<Integer> favourites = new HashSet<>();
		for (int m = 0; m < 20; m++) {
			favourites.add(Individuals.rendition(new UUID(random.nextLong(), random.nextLong()), 0.01f, 0, 12));
		}
		assertTrue(favourites.size() > 4, "favourites " + favourites);
	}
}
