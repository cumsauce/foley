package io.github.cumsauce.foley.audio;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.HashSet;
import java.util.Set;
import org.junit.jupiter.api.Test;

/** The per-biome rosters (M13), as a static table: what each biome can pick. */
class BiomeRosterTest {
	@Test
	void plainsPicksOnlyItsRoster() {
		Set<Integer> picked = new HashSet<>();
		for (int i = 0; i < 200; i++) picked.add(BiomeRoster.pick("minecraft:plains", i / 200f, false));
		// western_meadowlark 7, horned_lark 8, dickcissel 9, bobolink 10.
		for (int species : picked) assertTrue(species >= 7 && species <= 10, "plains picked " + species);
		assertEquals(4, picked.size(), "every plains species is reachable");
	}

	@Test
	void taigaSoundsLikeATaiga() {
		Set<Integer> picked = new HashSet<>();
		for (int i = 0; i < 200; i++) picked.add(BiomeRoster.pick("minecraft:taiga", i / 200f, false));
		// boreal_owl 0 through varied_thrush 6 — a taiga never picks a jungle bird.
		for (int species : picked) assertTrue(species >= 0 && species <= 6, "taiga picked " + species);
	}

	@Test
	void rarerSpeciesAreGated() {
		// Savanna: kori_bustard (22) is rare; jagged_peaks: golden_eagle (41), alpine_chough (42).
		Set<Integer> savanna = new HashSet<>();
		for (int i = 0; i < 500; i++) savanna.add(BiomeRoster.pick("minecraft:savanna", i / 500f, false));
		assertTrue(!savanna.contains(22), "kori bustard gated");
		savanna.clear();
		for (int i = 0; i < 500; i++) savanna.add(BiomeRoster.pick("minecraft:savanna", i / 500f, true));
		assertTrue(savanna.contains(22), "kori bustard reachable when rare included");
		Set<Integer> peaks = new HashSet<>();
		for (int i = 0; i < 500; i++) peaks.add(BiomeRoster.pick("minecraft:jagged_peaks", i / 500f, false));
		assertTrue(!peaks.contains(42) && !peaks.contains(43), "golden eagle and alpine chough gated");
	}

	@Test
	void theBeachKeepsItsGull() {
		Set<Integer> beach = new HashSet<>();
		for (int i = 0; i < 500; i++) beach.add(BiomeRoster.pick("minecraft:beach", i / 500f, false));
		assertTrue(beach.contains(53), "herring gull on the beach");
		assertTrue(beach.contains(45) && beach.contains(46), "tern and sanderling too");
	}

	@Test
	void biomesWithoutARosterAreSilent() {
		assertEquals(-1, BiomeRoster.pick("minecraft:ocean", 0.5f, true));
		assertEquals(-1, BiomeRoster.pick("minecraft:river", 0.5f, true));
		assertEquals(-1, BiomeRoster.pick("minecraft:the_nether", 0.5f, true));
		assertEquals(-1, BiomeRoster.pick(null, 0.5f, true));
	}

	@Test
	void everyRosterSpeciesExistsInTheCatalogue() {
		for (var roster : BiomeRoster.ROSTERS.values()) {
			for (var entry : roster) {
				assertTrue(Birds.ALL.contains(entry.species()), entry.species() + " missing from Birds.ALL");
			}
		}
		assertEquals(Birds.SPECIES, Birds.ALL.size());
		assertEquals(57, Birds.SPECIES, "48 catalogue species plus the nine originals");
	}
}
