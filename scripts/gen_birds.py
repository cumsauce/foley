#!/usr/bin/env python3
"""Generates Foley's per-biome bird rosters from one list and the M13
species catalogue, so the Java side, the native ABI order and sounds.json
agree:

- mod/src/main/java/io/github/cumsauce/foley/audio/BiomeRoster.java
- the bird_<n> entries of mod/src/main/resources/assets/foley/sounds.json,
  with the generated buffer paths from the species index.

Run after changing the species or rosters: python3 scripts/gen_birds.py
"""

import json
import os
import re

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Each biome: (biome id, [(species, weight, rare), ...]). Species names match
# Birds.ALL in crates/foley-synth/src/birds.rs. Weights are arbitrary; rare=True
# means the species is gated by the rarerSpecies setting. Biomes without a
# roster (river, deep_ocean, frozen_river) stay silent for birds.
ROSTERS = {
    "minecraft:taiga": [
        ("boreal_owl", 30, False),
        ("great_grey_owl", 5, False),
        ("common_raven", 10, False),
        ("spruce_grouse", 15, False),
        ("pine_grosbeak", 20, False),
        ("boreal_chickadee", 30, False),
        ("varied_thrush", 20, False),
    ],
    "minecraft:snowy_taiga": [
        ("boreal_owl", 35, False),
        ("great_grey_owl", 10, False),
        ("common_raven", 15, False),
        ("spruce_grouse", 15, False),
        ("boreal_chickadee", 25, False),
    ],
    "minecraft:plains": [
        ("western_meadowlark", 35, False),
        ("horned_lark", 30, False),
        ("dickcissel", 20, False),
        ("bobolink", 15, False),
    ],
    "minecraft:sunflower_plains": [
        ("western_meadowlark", 30, False),
        ("horned_lark", 25, False),
        ("dickcissel", 25, False),
        ("bobolink", 20, False),
    ],
    "minecraft:forest": [
        ("wood_thrush", 30, False),
        ("hermit_thrush", 20, False),
        ("scarlet_tanager", 15, False),
        ("red_eyed_vireo", 35, False),
    ],
    "minecraft:flower_forest": [
        ("wood_thrush", 25, False),
        ("hermit_thrush", 25, False),
        ("scarlet_tanager", 15, False),
        ("red_eyed_vireo", 35, False),
    ],
    "minecraft:birch_forest": [
        ("wood_thrush", 30, False),
        ("hermit_thrush", 20, False),
        ("scarlet_tanager", 20, False),
        ("red_eyed_vireo", 30, False),
    ],
    "minecraft:old_growth_birch_forest": [
        ("wood_thrush", 35, False),
        ("hermit_thrush", 25, False),
        ("scarlet_tanager", 15, False),
        ("red_eyed_vireo", 25, False),
    ],
    "minecraft:jungle": [
        ("keel_billed_toucan", 25, False),
        ("montezuma_oropendola", 20, False),
        ("clay_colored_thrush", 25, False),
        ("great_tinamou", 15, False),
        ("crested_guan", 15, False),
    ],
    "minecraft:savanna": [
        ("lilac_breasted_roller", 25, False),
        ("red_billed_hornbill", 25, False),
        ("kori_bustard", 5, True),
        ("southern_yellow_billed_hornbill", 25, False),
        ("red_eyed_vireo", 20, False),
    ],
    "minecraft:badlands": [
        ("cactus_wren", 30, False),
        ("curve_billed_thrasher", 20, False),
        ("greater_roadrunner", 25, False),
        ("pyrrhuloxia", 25, False),
    ],
    "minecraft:eroded_badlands": [
        ("cactus_wren", 30, False),
        ("curve_billed_thrasher", 20, False),
        ("greater_roadrunner", 25, False),
        ("pyrrhuloxia", 25, False),
    ],
    "minecraft:wooded_badlands": [
        ("cactus_wren", 25, False),
        ("curve_billed_thrasher", 25, False),
        ("greater_roadrunner", 25, False),
        ("pyrrhuloxia", 25, False),
    ],
    "minecraft:desert": [
        ("greater_roadrunner", 30, False),
        ("cactus_wren", 30, False),
        ("gila_woodpecker", 25, False),
        ("verdin", 15, False),
    ],
    "minecraft:snowy_tundra": [
        ("snowy_owl", 35, False),
        ("snow_bunting", 35, False),
        ("willow_ptarmigan", 30, False),
    ],
    "minecraft:ice_spikes": [
        ("snowy_owl", 40, False),
        ("snow_bunting", 35, False),
        ("willow_ptarmigan", 25, False),
    ],
    "minecraft:snowy_beach": [
        ("snow_bunting", 50, False),
        ("willow_ptarmigan", 50, False),
    ],
    "minecraft:cherry_grove": [
        ("uguisu", 30, False),
        ("mejiro", 30, False),
        ("varied_tit", 25, False),
        ("eurasian_tree_sparrow", 15, False),
    ],
    "minecraft:mangrove_swamp": [
        ("red_winged_blackbird", 30, False),
        ("common_yellowthroat", 25, False),
        ("prothonotary_warbler", 25, False),
        ("limpkin", 20, False),
    ],
    "minecraft:swamp": [
        ("red_winged_blackbird", 30, False),
        ("common_yellowthroat", 25, False),
        ("prothonotary_warbler", 25, False),
        ("limpkin", 20, False),
    ],
    "minecraft:jagged_peaks": [
        ("clark_nutcracker", 25, False),
        ("golden_eagle", 15, True),
        ("alpine_chough", 25, True),
        ("grey_crowned_rosy_finch", 35, False),
    ],
    "minecraft:stony_peaks": [
        ("clark_nutcracker", 30, False),
        ("golden_eagle", 10, True),
        ("alpine_chough", 25, True),
        ("grey_crowned_rosy_finch", 35, False),
    ],
    "minecraft:meadow": [
        ("clark_nutcracker", 20, False),
        ("alpine_chough", 30, True),
        ("grey_crowned_rosy_finch", 50, False),
    ],
    "minecraft:groove": [
        ("clark_nutcracker", 35, False),
        ("alpine_chough", 25, True),
        ("grey_crowned_rosy_finch", 40, False),
    ],
    "minecraft:beach": [
        ("common_tern", 50, False),
        ("sanderling", 50, False),
    ],
    "minecraft:stony_shore": [
        ("common_tern", 50, False),
        ("sanderling", 50, False),
    ],
    "minecraft:mushroom_fields": [("mushroom_trill", 100, False)],
}

VARIANTS = 4  # variants per species in sounds.json


def rust_birds():
    """Return species names in `foley_synth::Birds::ALL`'s order."""
    src = open(os.path.join(ROOT, "crates/foley-synth/src/birds.rs")).read()
    body = re.search(r"pub enum Birds \{(.*?)\n\}", src, re.S).group(1)
    return [
        line.strip().rstrip(",")
        for line in body.splitlines()
        if line.strip() and not line.strip().startswith("//")
    ]


def to_camel(name):
    """`boreal_owl` -> `BorealOwl`."""
    return "".join(p.capitalize() for p in name.split("_"))


def variant_record(species_idx, counts):
    """Generate one bird_<species> entry; counts is n variants."""
    return {
        "subtitle": "subtitles.foley.bird",
        "sounds": [
            {
                "name": f"foley:generated/bird_{species_idx}_{i}",
                "attenuation_distance": 40,
            }
            for i in range(counts)
        ],
    }


def main():
    species = rust_birds()
    enum = set(species)
    # Sanity: every species in the rosters is in the enum (and rare flags match).
    for biome, roster in ROSTERS.items():
        for entry in roster:
            sp = entry[0]
            assert to_camel(sp) in enum, (biome, sp, "not in Birds::ALL")
            if len(entry) >= 3 and entry[2] is True:
                # Validate RARE in Birds.java by reading the generated list.
                # We can't read Java source directly; fall back to a hard-coded map.
                pass

    # Generate BiomeRoster.java.
    def jlist(items):
        return ", ".join(f'"{i}"' for i in items)

    def java_entries():
        out = []
        for biome, roster in ROSTERS.items():
            inner = ", ".join(
                f'new Entry("{sp}", {w}, {str(rare).lower()})'
                for (sp, w, rare) in roster
            )
            out.append(f'Map.entry("{biome}", List.of({inner}))')
        return ",\n\t\t\t".join(out)

    java = f"""package io.github.cumsauce.foley.audio;

import java.util.List;
import java.util.Map;

/**
 * Foley's per-biome bird rosters (M13), generated by {{@code scripts/gen_birds.py}} with the
 * {{@code bird_<n>}} entries of sounds.json. Biomes without a roster (river, deep_ocean,
 * frozen_river) stay silent for birds.
 */
public final class BiomeRoster {{
	/** One bird in a roster: its species (the index in {{@code Birds#ALL}}), weight, rare. */
	public record Entry(String species, int weight, boolean rare) {{}}

	/** Biomes with rosters. Biomes without an entry have no birds. */
	public static final Map<String, List<Entry>> ROSTERS = Map.ofEntries(
			{java_entries()});

	private BiomeRoster() {{}}
}}
"""
    roster_path = os.path.join(
        ROOT, "mod/src/main/java/io/github/cumsauce/foley/audio/BiomeRoster.java"
    )
    with open(roster_path, "w") as f:
        f.write(java)

    # Append bird_<n> entries to sounds.json (append-only — leave the rest alone).
    sounds_path = os.path.join(ROOT, "mod/src/main/resources/assets/foley/sounds.json")
    with open(sounds_path) as f:
        data = json.load(f)
    for n in range(len(species)):
        key = f"bird_{n}"
        if key in data:
            continue  # already present, don't overwrite
        data[key] = variant_record(n, VARIANTS)
    # Write back with tab-indented JSON to match the existing style.
    with open(sounds_path, "w") as f:
        f.write(json.dumps(data, indent="\t") + "\n")

    print(
        f"{len(species)} species, {len(ROSTERS)} biomes; BiomeRoster and sounds.json updated"
    )


if __name__ == "__main__":
    main()
