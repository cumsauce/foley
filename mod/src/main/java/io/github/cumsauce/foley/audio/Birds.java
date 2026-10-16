package io.github.cumsauce.foley.audio;

import java.util.List;

/** Foley's M13 bird species, in {@code foley_synth::Birds::ALL}'s order. */
public final class Birds {
	public static final List<String> ALL = List.of(
			"boreal_owl", "great_grey_owl", "common_raven", "spruce_grouse", "pine_grosbeak",
			"boreal_chickadee", "varied_thrush",
			"western_meadowlark", "horned_lark", "dickcissel", "bobolink",
			"wood_thrush", "hermit_thrush", "scarlet_tanager", "red_eyed_vireo",
			"keel_billed_toucan", "montezuma_oropendola", "clay_colored_thrush",
			"great_tinamou", "crested_guan",
			"lilac_breasted_roller", "red_billed_hornbill", "kori_bustard",
			"southern_yellow_billed_hornbill",
			"cactus_wren", "curve_billed_thrasher", "greater_roadrunner", "pyrrhuloxia",
			"gila_woodpecker", "verdin",
			"snowy_owl", "snow_bunting", "willow_ptarmigan",
			"uguisu", "mejiro", "varied_tit", "eurasian_tree_sparrow",
			"red_winged_blackbird", "common_yellowthroat", "prothonotary_warbler", "limpkin",
			"clark_nutcracker", "golden_eagle", "alpine_chough", "grey_crowned_rosy_finch",
			"common_tern", "sanderling",
			"mushroom_trill");

	/**
	 * The rare-gated species, in {@code ALL}'s order: {@code true} means the
	 * species is behind the {@code rarerSpecies} setting (Kori bustard, golden
	 * eagle, alpine chough).
	 */
	public static final List<Boolean> RARE = List.of(
			false, false, false, false, false, false, false,
			false, false, false, false,
			false, false, false, false,
			false, false, false, false, false,
			false, false, true, false,
			false, false, false, false,
			false, false,
			false, false, false,
			false, false, false, false,
			false, false, false, false,
			false, true, true, false,
			false, false,
			false);

	public static final int SPECIES = ALL.size();

	private Birds() {}
}
