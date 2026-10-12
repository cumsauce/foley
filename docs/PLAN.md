# Foley — Plan

## Goal

A client-only Fabric mod for Minecraft 26.3 that adds sound effects and ambience **synthesized at runtime from physical models**, not played from recordings. Every sound reacts to its situation: what a raindrop lands on, how fast the wind is and what it blows past, how big a bubble is. It runs alongside Earshot and Yak VC, and needs neither.

## Is it possible? Yes

Checked against the 26.3 game and Fabric API (2026-10-12):

- **Custom audio into the game's sound engine.** Fabric's sound API lets a `SoundInstance` supply its own `AudioStream` (`FabricSoundInstance.getAudioStream`). The game then plays our PCM through its own OpenAL channels, with its volume sliders, subtitles, pausing and positional falloff.
- **Earshot shapes it for free.** Earshot hooks every game channel (`ChannelMixin`), so Foley's sounds get occlusion, sounds round corners, the room's reverb, echoes and HRTF with no integration work. Without Earshot, they play as vanilla sounds do.
- **Yak VC is unaffected:** voice goes through Earshot's own sources or Yak VC's speakers, never through Foley.
- **Native DSP.** The same Rust-plus-FFM setup as Earshot (a cdylib, natives in the jar) runs the synthesis off the game's threads.

**The one real constraint: streams.** The game allows at most 8 streaming channels at once, music and records included. Foley therefore uses two paths:

1. **Beds** (rain, wind, a river): at most 3–4 long-running streams, each a mix of many procedural events, with positions moved as the player does.
2. **One-shots** (a drip, a crackle, a footstep variation, a branch creak): rendered into ordinary OpenAL buffers off-thread, and handed to the game's static-sound cache under generated names, so they play from its pool of about 255 static sources like any other sound. A mixin on `SoundBufferLibrary.getCompleteBuffer` serves them.

**"Physically accurate."** Everything is driven by physical models: bubble resonance, modal vibration, aeroacoustic tones, turbulence spectra, published drop statistics. That is the standard of procedural audio (Farnell's *Designing Sound*, van den Doel's physically based models). It is not a wave simulation of every raindrop: each model is the physics that dominates what you hear, with parameters from the literature and from the world.

## Catalogue (by milestone)

| Sound | Model | Driven by |
| --- | --- | --- |
| **Rain** (bed) | Drops as a Poisson process sized by the Marshall–Palmer distribution for the rain's intensity. Each impact: on water, the Minnaert bubble resonance f = (1/2πr)·√(3γp/ρ) with a rising chirp; on solids, a modal "tick" from the surface material; on leaves, a short damped plate | Rain/thunder state; the blocks the rain lands on near the player (sampled from the world, or from Earshot's mirror when present) |
| **Wind** (bed) | Turbulence as filtered noise with a von Kármán spectrum and gusts; Aeolian tones off edges (Strouhal f ≈ 0.2·U/d) for fences, bars and leaves; whistle through narrow openings | Height above ground, exposure (sky openness), biome, weather, thunder |
| **Thunder** | Ribner–Roy tortuous channel: N-waves from a fractal lightning channel, summed by path length to the listener, so near bolts crack and far ones rumble | Bolt position and distance (replaces vanilla thunder) |
| **Water** (bed and positional) | Bubble-population synthesis: streams and waterfalls as many Minnaert bubbles, with rate and sizes from flow (falling water vs level) | Water blocks and flow nearby |
| **Cave drips** | A falling drop: impact plus a single Minnaert bubble when it lands in water, with pitch from drop size | Dripstone, water above air, cave enclosure |
| **Fire** | Combustion noise (low-passed turbulence) plus Poisson crackles (resin pops as short broadband bursts) | Fire and campfire blocks, burning entities |
| **Lava** | Viscous bubble bursts: slow, low Minnaert-like bubbles with heavy damping, occasional pops | Lava blocks nearby |
| **Impacts and breaking** | Modal synthesis: each material's modes (frequencies, damping, gains) from its density, stiffness and the block's size; fracture as a burst of small modal fragments (glass shatter, stone crack) | Block breaks, hits, landings |
| **Footsteps** (optional layer) | Modal contact on the block's material, plus granular crunch for snow, gravel and sand (the grain count follows weight and speed) | Player and mob movement |
| **Ambient life** | Insects as FM chirps with temperature-dependent rate (Dolbear's law for crickets: chirps/min ≈ 7·T + 30, T in °C); birdsong from a syrinx-inspired FM model | Biome, time of day, temperature, weather |

## With or instead of vanilla (decided 2026-10-12)

- **Soundscapes are added on top.** Wind, ambient life and cave atmosphere layer over the game's own sounds and music; vanilla has nothing for them anyway.
- **Continuous environmental sounds Foley models replace vanilla's:** rain, flowing water and waterfalls, fire, lava, thunder. Two rains at once would be wrong, and Foley's version follows the world.
- **One-shots keep vanilla's:** mob sounds (a cow's moo), item and UI sounds, and the like. Foley's impacts and breaking layer under them, without replacing them.
- **Mob voices** can later be made procedurally too (M6), as an option you select, off by default.

Every replacement can still be switched back to vanilla, per sound, in the settings.

## Architecture

```
Fabric mod (Java)                                   foley-ffi (Rust cdylib, FFM)
├─ Scene sampler: weather, wind exposure,           ├─ Synth engine: oscillators, noise, filters,
│  surfaces round the player, water/fire/lava       │  modal banks, bubble model, granular, envelopes
│  near, biome, time  ──────────────────────────────▶  ├─ Beds: rain/wind/water mixers, 48 kHz mono
├─ Beds: ≤ 4 FabricSoundInstances streaming         │  per bed, pulled by the game's stream reader
│  from native beds                                 ├─ One-shot renderer (worker threads): renders
├─ One-shots: SoundBufferLibrary mixin serves       │  variations into PCM, cached and recycled
│  generated buffers under foley:<model>/<seed>     └─ Physics tables: materials (density, Young's
└─ Settings screen, presets, toggles per sound         modulus, damping), bubble and drop statistics
```

- **Materials:** the same block → material classification idea as Earshot's (sound type, tags, shape), with mechanical properties added (density, stiffness, loss factor) for modal synthesis. When Earshot is present, its world mirror can be shared through Fabric's object share, rather than copying the world twice.
- **Variation:** every one-shot is generated from a seed, so no two drips or crackles are the same, and none ever repeats exactly.
- **CPU budget:** beds about 2–4 % of a core, and one-shots rendered ahead on a worker (about 0.1 ms per 100 ms of audio). A preset system like Earshot's (Low to Ultra) scales drop counts, bubble populations and model detail.

## M0 results (2026-10-12)

- **Both paths play in the real game**, checked by the client gametest on the M3 VM:
  - a synthesized stream (rain on water) through `FabricSoundInstance.getAudioStream`, read buffer after buffer by the game's stream thread;
  - a generated one-shot (a drip) served as an ordinary buffer by a `SoundBufferLibrary.getCompleteBuffer` mixin, from `foley:generated/drip_0..15` in sounds.json. A `SoundManager.validateSoundResource` mixin keeps these names: the game would drop them for having no file.
- **Audio format:** 48 kHz mono 16-bit signed PCM. The game's OpenAL streams take only 8- and 16-bit.
- **Physics so far:**
  - a bubble's Minnaert resonance, f = (1/2πr)·√(3γp/ρ) (3.29 kHz at 1 mm);
  - its ring decaying at π·f·δ with Devin's total damping δ ≈ 0.045 (about 15 ms for 1 mm), rising in pitch as it surfaces;
  - rain as Poisson drops, each trapping a bubble of 0.2–1.5 mm, plus splash hiss.
  The van den Doel damping formula as first written gave 4 ms rings, several times too short against measured δ, so measured damping is used.
- **Cost** (bed_bench, M3 VM, one core): 100 drops/s 0.1 %, 400 drops/s 0.3 %, 1,500 drops/s 1.2 %, 5,000 drops/s (a downpour) 3.7 %.
- **To check by ear:** that Earshot shapes both. It hooks every game channel, and these play through the game's channels like any sound.

## M1 results (2026-10-12)

- **Rain replaces the game's.** While Foley's rain plays, a `SoundEngine` mixin skips `weather.rain` and `weather.rain.above`. The client gametest checks that it starts with `weather rain`, keeps being rendered, and that the game's rain sounds were skipped.
- **Rain follows the world.** Every half second the scene is sampled:
  - the rate: rain level × (10 + 40 × thunder level) mm/h;
  - what it lands on: the top block of every second column within 8 blocks where it rains (not snows), weighted 1/(1 + d²), classified into water, stone, leaves, wood, soft ground, glass and metal;
  - the roof: the first solid block above the ear within 16 blocks, which then drums instead of the open-air patter. Deep inside with nothing open to the sky, rain isn't heard.
  The gametest checks a superflat's grass reads as soft ground, and that a roof of oak planks reads as wood.
- **Rain physics:** drops by Marshall–Palmer (Λ = 4.1·R^−0.21 /mm, N₀ = 8000 /m³/mm), about 4,500 a second per m² at 10 mm/h, landing at Gunn–Kinzer terminal velocity (9.65 − 10.3·e^−0.6D m/s). Near drops (half a m²) are each synthesized; the rest is a texture whose loudness follows the flux. On water, drops of 0.8–1.1 mm trap the 14 kHz bubbles Medwin and Pumphrey measured; others trap one less often. On solids each impact excites a material's modes (damped, with an impact force from the drop's momentum).
- **Wind** layers on the game's sounds, as decided:
  - a 10 m reference speed from the weather (5 m/s clear, 9 in rain, 16 in a thunderstorm), carried to the ear by the log wind profile U(z) = U₁₀·ln(z/z₀)/ln(10/z₀), with z the height above the ground within 24 blocks and z₀ from 0.03 m (open) to 1 m (woods). So it rises on a hill top and drops in a forest;
  - gusts as an Ornstein–Uhlenbeck process with turbulence intensity 0.25 and a 4 s time scale;
  - turbulence heard as band-passed noise whose amplitude goes as U³ (aerodynamic noise power ∝ U⁶) and whose pitch rises with speed; leaves rustling as bursts at a rate ∝ U²;
  - exposure: the sky over the ear counts half, the open columns round it the rest. Shut in, only a low, quiet part gets through;
  - the listener's own motion adds to it (flying with elytra is loud). No wind in a dimension without a sky.
  The gametest checks it blows in the open on a superflat, and that a roof lowers the exposure.
- **Cost** (bed_bench, M3 VM, one core): rain 0.2 % at 2 mm/h, 0.4 % at 10 and 50 mm/h (near drops are capped, the rest is texture). Wind 0.03 % at any speed. Phasor oscillators cut rain about 12× from M0.
- **To check by ear:** standing under a tree, on stone and by a lake in the rain; the wind on a mountain top against a valley.

## Milestones

0. **M0 Groundwork.** Repo, Rust workspace, FFM bridge, CI and natives packaging, all copied from Earshot. Spike: one procedural `FabricSoundInstance` stream (a sine, then filtered noise), and one generated buffer through the `SoundBufferLibrary` mixin. Measure CPU on the M3. **Exit:** both paths play in game, and Earshot shapes them.
1. **M1 Rain and wind beds.** Marshall–Palmer drops on sampled surfaces with the Minnaert water model; turbulence wind with exposure. **Exit:** standing under a tree, on stone and by a lake in the rain sound clearly different; wind rises on a mountain top.
2. **M2 Water, drips, lava, fire.** Bubble populations, drips, viscous lava, combustion with crackles.
3. **M3 Thunder.** The Ribner–Roy channel model, replacing vanilla thunder (Earshot's thunder delay still applies).
4. **M4 Modal impacts and breaking.** Material modal tables, block breaking and landing, glass and stone fracture.
5. **M5 Ambient life, footsteps, settings, first prerelease.**
6. **M6 Procedural mob voices (optional, off by default).** Vocal-tract models for the animals and monsters: source-filter synthesis, with formants and pitch from each mob's size and kind (a cow's low moo, a chicken's cluck, a villager's "hmm"). Each mob can be switched between Foley's voice and vanilla's.

## Risks

| Risk | Mitigation |
| --- | --- |
| 8-stream limit | Beds stay at 4 or fewer; everything short goes through generated static buffers. |
| Synthesized sound can sound "synthetic" | Physical models with measured parameters, per-event randomization, and A/B against vanilla with a bypass key. Each sound can be switched back to vanilla. |
| Clashing with resource packs that replace the same sounds | Replaced vanilla sounds are opt-in per sound. |
| CPU on weaker machines | Presets, plus a cap on drops, bubbles and voices per second. |

## Decided

1. **Name:** Foley.
2. **Vanilla:** soundscapes on top; rain, water, fire, lava and thunder replaced; one-shots such as mob sounds kept (see above).
3. **Order:** rain and wind first, the most audible all the time.
