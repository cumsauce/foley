#!/usr/bin/env python3
"""Generates Foley's mob voice tables from one list, so the Java side, the
native ABI order and sounds.json agree:

- mod/src/main/java/io/github/cumsauce/foley/audio/VoiceTable.java
- the voice_* entries of mod/src/main/resources/assets/foley/sounds.json,
  with the game's own subtitles (read from the game's sounds.json in Loom's
  asset cache).

Run it after changing the mobs or calls: python3 scripts/gen_voices.py
"""
import glob, json, os, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# The native mobs and calls, in crates/foley-synth/src/voice/mod.rs's order.
MOBS = ["cow", "pig", "sheep", "chicken", "villager", "zombie", "wolf", "cat", "husk", "drowned",
        "zombie_villager", "skeleton", "stray", "bogged", "spider", "cave_spider", "creeper", "bat",
        "horse", "donkey", "mule", "goat", "llama", "rabbit", "fox", "parrot", "bee", "polar_bear",
        "panda", "frog", "camel", "armadillo", "turtle", "ocelot", "wandering_trader", "iron_golem",
        "snow_golem", "enderman", "witch", "slime", "slime_small", "phantom", "pillager", "vindicator",
        "evoker", "illusioner", "ravager", "vex", "guardian", "elder_guardian", "silverfish", "endermite"]
CALLS = ["ambient", "hurt", "death", "growl", "pant", "whine", "hiss", "purr", "beg", "primed",
         "ambient_water", "hurt_water", "death_water", "breath", "breath_heavy", "ambient_near", "angry",
         "snort", "scream", "screech", "warning", "sneeze", "worried", "aggressive", "yes", "no", "trade",
         "celebrate", "stare", "squish", "jump", "attack", "roar", "stunned", "charge", "swoop", "flap",
         "flop", "ambient_land", "loop", "loop_aggressive", "sleep", "ambient_baby"]
# The settings' groups, in the order of their bits, and the mobs each covers.
GROUPS = {
    "cow": ["cow"], "pig": ["pig"], "sheep": ["sheep"], "chicken": ["chicken"],
    "villager": ["villager", "wandering_trader"], "zombie": ["zombie", "husk", "drowned", "zombie_villager"],
    "wolf": ["wolf"], "cat": ["cat"], "skeleton": ["skeleton", "stray", "bogged"],
    "spider": ["spider", "cave_spider"], "creeper": ["creeper"], "bat": ["bat"],
    "horse": ["horse", "donkey", "mule"], "goat": ["goat"], "llama": ["llama"], "rabbit": ["rabbit"],
    "fox": ["fox"], "parrot": ["parrot"], "bee": ["bee"], "polar_bear": ["polar_bear"], "panda": ["panda"],
    "frog": ["frog"], "camel": ["camel"], "armadillo": ["armadillo"], "turtle": ["turtle"], "ocelot": ["ocelot"],
    "golem": ["iron_golem", "snow_golem"], "enderman": ["enderman"], "witch": ["witch"],
    "slime": ["slime", "slime_small"], "phantom": ["phantom"],
    "illager": ["pillager", "vindicator", "evoker", "illusioner"], "ravager": ["ravager"], "vex": ["vex"],
    "guardian": ["guardian", "elder_guardian"], "silverfish": ["silverfish", "endermite"],
}
AHD = {"ambient": "ambient", "hurt": "hurt", "death": "death"}
ZOMBIE = dict(AHD)
# The game's mobs: their call names (after entity.<mob>.) and Foley's call, as "call" or "mob:call".
GAME = {
    "cow": AHD, "pig": AHD, "sheep": AHD, "chicken": AHD,
    "villager": {**AHD, "yes": "yes", "no": "no", "trade": "trade", "celebrate": "celebrate"},
    "zombie": ZOMBIE, "husk": ZOMBIE, "zombie_villager": ZOMBIE,
    "drowned": {**AHD, "ambient_water": "ambient_water", "hurt_water": "hurt_water", "death_water": "death_water"},
    "wolf": {**AHD, "growl": "growl", "pant": "pant", "whine": "whine"},
    "cat": {**AHD, "hiss": "hiss", "purr": "purr", "beg_for_food": "beg", "purreow": "beg", "stray_ambient": "ambient"},
    "skeleton": AHD, "stray": AHD, "bogged": AHD, "spider": AHD, "bat": AHD,
    "creeper": {"primed": "primed", "hurt": "hurt", "death": "death"},
    "horse": {**AHD, "angry": "angry", "breathe": "snort"},
    "donkey": {**AHD, "angry": "angry"}, "mule": {**AHD, "angry": "angry"},
    "goat": {**AHD, "screaming.ambient": "scream", "screaming.hurt": "hurt", "screaming.death": "death"},
    "llama": {**AHD, "angry": "angry"},
    "rabbit": AHD,
    "fox": {**AHD, "aggro": "aggressive", "screech": "screech", "sniff": "snort", "sleep": "sleep"},
    "parrot": AHD,
    "bee": {"loop": "loop", "loop_aggressive": "loop_aggressive", "hurt": "hurt", "death": "death"},
    "polar_bear": {**AHD, "ambient_baby": "ambient_baby", "warning": "warning"},
    "panda": {**AHD, "aggressive_ambient": "aggressive", "worried_ambient": "worried", "sneeze": "sneeze"},
    "frog": AHD, "camel": AHD,
    "armadillo": {**AHD, "hurt_reduced": "hurt"},
    "turtle": {"ambient_land": "ambient_land", "hurt": "hurt", "death": "death"},
    "ocelot": AHD,
    "wandering_trader": {**AHD, "yes": "yes", "no": "no", "trade": "trade"},
    "iron_golem": {"hurt": "hurt", "damage": "hurt", "death": "death"},
    "snow_golem": AHD,
    "enderman": {**AHD, "stare": "stare", "scream": "scream"},
    "witch": {**AHD, "celebrate": "celebrate"},
    "slime": {"squish": "squish", "jump": "jump", "attack": "attack", "hurt": "hurt", "death": "death",
              "squish_small": "slime_small:squish", "jump_small": "slime_small:jump",
              "hurt_small": "slime_small:hurt", "death_small": "slime_small:death"},
    "phantom": {**AHD, "swoop": "swoop", "flap": "flap"},
    "pillager": {**AHD, "celebrate": "celebrate"}, "vindicator": {**AHD, "celebrate": "celebrate"},
    "evoker": {**AHD, "celebrate": "celebrate"}, "illusioner": AHD,
    "ravager": {**AHD, "roar": "roar", "stunned": "stunned", "celebrate": "celebrate"},
    "vex": {**AHD, "charge": "charge"},
    "guardian": {**AHD, "ambient_land": "ambient_land", "flop": "flop", "hurt_land": "hurt", "death_land": "death"},
    "elder_guardian": {**AHD, "ambient_land": "ambient_land", "flop": "flop", "hurt_land": "hurt", "death_land": "death"},
    "silverfish": AHD, "endermite": AHD,
}
# Calls Foley makes itself (not in place of a game sound).
EXTRA = {m: ["breath", "breath_heavy", "ambient_near", "growl"] for m in ["zombie", "husk", "drowned", "zombie_villager"]}
LOOPS = {"loop", "loop_aggressive"}
EXTRA_SUBTITLES = {"breath": "subtitles.foley.breath", "breath_heavy": "subtitles.foley.breath",
                   "growl": "subtitles.foley.snarl"}


def vanilla_subtitles():
    indexes = sorted(glob.glob(os.path.expanduser("~/.gradle/caches/fabric-loom/assets/indexes/*.json")), key=os.path.getmtime)
    if not indexes:
        return {}
    objects = json.load(open(indexes[-1]))["objects"]
    h = objects["minecraft/sounds.json"]["hash"]
    sounds = json.load(open(os.path.expanduser(f"~/.gradle/caches/fabric-loom/assets/objects/{h[:2]}/{h}")))
    return {k: v.get("subtitle") for k, v in sounds.items()}


def rust_enum(name):
    import re
    source = open(os.path.join(ROOT, "crates/foley-synth/src/voice/mod.rs")).read()
    body = re.search(r"pub enum " + name + r" \{(.*?)\n\}", source, re.S).group(1)
    names = [line.strip().rstrip(",") for line in body.splitlines() if line.strip() and not line.strip().startswith("//")]
    return [re.sub(r"(?<!^)(?=[A-Z])", "_", n).lower() for n in names]


def main():
    assert rust_enum("Mob") == MOBS, "MOBS doesn't match voice/mod.rs's Mob"
    assert rust_enum("Call") == CALLS, "CALLS doesn't match voice/mod.rs's Call"
    for game, calls in GAME.items():
        for call, target in calls.items():
            mob, ours = target.split(":") if ":" in target else (game, target)
            assert mob in MOBS and ours in CALLS, (game, call, target)
    subtitles = vanilla_subtitles()
    # Every (native mob, call) Foley needs, with a subtitle.
    events = {}
    for game, calls in GAME.items():
        for call, target in calls.items():
            mob, ours = target.split(":") if ":" in target else (game, target)
            sub = subtitles.get(f"entity.{game}.{call}") or subtitles.get(f"entity.{game}.ambient")
            events.setdefault((mob, ours), sub)
    for mob, calls in EXTRA.items():
        for call in calls:
            events.setdefault((mob, call), EXTRA_SUBTITLES.get(call) or subtitles.get(f"entity.{mob}.ambient"))

    path = os.path.join(ROOT, "mod/src/main/resources/assets/foley/sounds.json")
    sounds = {k: v for k, v in json.load(open(path)).items() if not k.startswith("voice_")}
    for (mob, call), sub in sorted(events.items()):
        n = 4 if call in LOOPS else 12
        entry = {"sounds": [f"foley:generated/voice_{mob}_{call}_{i}" for i in range(n)]}
        if sub:
            entry = {"subtitle": sub, **entry}
        sounds[f"voice_{mob}_{call}"] = entry
    with open(path, "w") as f:
        f.write(json.dumps(sounds, indent="\t") + "\n")

    def jlist(items):
        return ", ".join(f'"{i}"' for i in items)

    def entry(key, value):
        return 'Map.entry("' + key + '", ' + value + ')'

    def quoted(text):
        return '"' + text + '"'

    game_entries = ",\n\t\t\t".join(
        entry(g, "Map.ofEntries(" + ", ".join(entry(c, quoted(t)) for c, t in calls.items()) + ")")
        for g, calls in GAME.items())
    group_of = ",\n\t\t\t".join(f'Map.entry("{m}", "{g}")' for g, ms in GROUPS.items() for m in ms)
    extra = ",\n\t\t\t".join(f'Map.entry("{m}", List.of({jlist(c)}))' for m, c in EXTRA.items())
    java = f"""package io.github.cumsauce.foley.audio;

import java.util.List;
import java.util.Map;

/**
 * Foley's mob voices, as tables. Generated by {{@code scripts/gen_voices.py}} with the voice_* entries of sounds.json;
 * edit that, not this.
 */
public final class VoiceTable {{
	/** The native mobs, in the C ABI's order. */
	public static final List<String> MOBS = List.of({jlist(MOBS)});
	/** The native calls, in the C ABI's order. */
	public static final List<String> CALLS = List.of({jlist(CALLS)});
	/** The settings' groups, in the order of their bits. */
	public static final List<String> GROUPS = List.of({jlist(GROUPS)});
	/** Each native mob's group. */
	public static final Map<String, String> GROUP_OF = Map.ofEntries(
			{group_of});
	/** The game's mobs: their call names (after {{@code entity.<mob>.}}) and Foley's call, {{@code call}} or {{@code mob:call}}. */
	public static final Map<String, Map<String, String>> GAME = Map.ofEntries(
			{game_entries});
	/** Calls Foley makes itself, beyond the game's. */
	public static final Map<String, List<String>> EXTRA = Map.ofEntries(
			{extra});

	private VoiceTable() {{}}
}}
"""
    out = os.path.join(ROOT, "mod/src/main/java/io/github/cumsauce/foley/audio/VoiceTable.java")
    with open(out, "w") as f:
        f.write(java)
    print(f"{len(events)} voice events, {len(MOBS)} mobs, {len(CALLS)} calls, {len(GROUPS)} groups; no subtitle for",
          sorted(f"{m}.{c}" for (m, c), s in events.items() if not s))


if __name__ == "__main__":
    main()
