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
| **Ambient life** | Crickets with temperature-dependent rate (Dolbear's law: chirps/min ≈ 7.2·T − 32, T in °C); birdsong from a syrinx model (a self-sustained labial oscillator); frogs as glottal pulses through a vocal-sac resonance | Biome, time of day, temperature, weather |

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

## M2 results (2026-10-12)

- **Beds are now generic** (ABI 4): `foley_bed_create(kind, ...)` and a scene of floats for each kind, and `foley_render_one_shot(kind, ...)`. Water, fire and lava are **placed beds**: one stream each, played from the power-weighted centre of what's heard and moved as that changes, so they pan and Earshot occludes them. Distance is in the scene's gain (2/d_eff for the power-weighted distance, as power falls as 1/d²), not OpenAL's falloff. With rain and wind, that is at most 5 of the game's 8 streams.
- **What's heard** (`Surroundings`): every block within 12 across and 8 up and down, skipping chunk sections with none of them (`maybeHas`):
  - water flowing at its surface;
  - a waterfall where it lands (1) and down its face (0.1);
  - lava open to the air;
  - fire (1) and a lit campfire (0.6).
  Still water is silent. The game's water, lava and fire ambience (`block.water.ambient`, `block.lava.ambient`, `block.lava.pop`, `block.fire.ambient`, `block.campfire.crackle`) is skipped from within that reach and kept beyond it.
- **Water:** bubble populations. A stream entrains 60 bubbles a second per block, 1 to 4 mm across (log-uniform; brooks peak near 1 to 3 kHz). A waterfall entrains 700 per block where it lands, sized by Deane and Stokes (N(a) ∝ a^−3/2 below the 1 mm Hinze scale and a^−10/3 above, so about 9 % are over 1 mm), over the low roar and spray of water striking water. Each bubble rings at its Minnaert frequency (M0). Past 3,000 a second, each one is louder instead (√ of the ratio, as incoherent sources add).
- **Fire:** combustion roar, low-passed lower the bigger the fire, puffing at f ≈ 1.5/√D Hz for a fire D m across (Cetegen and Ahmed). Crackles are a Poisson process, 7 a second per block, with sizes from P(E) ∝ E^−1.7 (crackling noise): mostly ticks of 0.2 ms, now and then a 2 ms pop.
- **Lava:** a rumble below 60 Hz, and bubbles bursting 0.35 a second per block. Each is 1 to 6 cm, and its cavity rings as a Helmholtz resonator, f = (c/2π)·√(A/(V·L)), with radiation Q = 2π·√(V(L/A)³). A 3 cm bubble opening 1.5 cm rings near 1.2 kHz with Q ≈ 9, halved for the melt's viscosity. Each has a thump below it and a sizzle of spatter.
- **Drips:** the game's dripstone drips are swapped for generated ones where the drop lands:
  - into water or a cauldron: the M0 bubble drip;
  - on stone: a 2 to 2.8 mm drop's tick through stone's modes, with a bright splash;
  - lava: a thud and a sizzle.
- The client gametest builds a waterfall, a fire on netherrack and a pool of lava round the player and checks each bed plays; then plays the game's dripstone drip and checks it was swapped.
- **Cost** (bed_bench, M3 VM, one core):
  - a stream of 20 blocks: 1.4 %;
  - with a 10-block waterfall: 1.8 %;
  - fire of 20 blocks: 0.03 %;
  - lava of 200 blocks: 0.1 %.

## M3 results (2026-10-12)

- **The Ribner–Roy model of thunder:**
  - **Channel:** a tortuous lightning channel, 2.5 to 4 km from the ground into the cloud, made of segments 8 to 38 m long. Each segment turns from the last by a mean of 16°, as Hill measured on photographs. There are 3 to 6 branches off the lower part at 0.3 of the energy, and 1 to 4 later strokes 30 to 100 ms apart down the main channel, weaker each time.
  - **N-wave:** each segment's shock relaxes into an N-wave. Its length starts at Few's 1/f_m = R₀/0.63c, with R₀ = √(E/πp) ≈ 1.3 m for 5×10⁵ J/m, and grows as r^¼ as a weak shock ages. Its amplitude goes as R₀·L/r.
  - **Clap or rumble:** each N-wave is averaged over the time sound takes along its segment, L·|cos φ|/c, as Ribner and Roy had it. Side-on segments clap and end-on ones rumble, so the channel's shape against the listener makes the thunder.
  - **Air:** ISO 9613-1 absorption over each segment's own distance takes the highs off far parts. The cutoff is where the air takes 6 dB, and two one-pole low-passes apply it. The formula is checked against ISO 9613-2's table: 5.0, 22.9 and 76.6 dB/km at 1, 4 and 8 kHz, at 20 °C and 70 %.
  - **Results:** a flash 300 m off first sounds after 0.87 s, as sound takes. It cracks, then rumbles for 9 s. One 4 km off is a tenth as loud at its peak, duller, and lasts 15 s.
- **In the game:** Foley strikes when a lightning bolt entity appears on the client, at the flash, and skips the game's thunder and impact sounds. Earshot delays the game's own thunder by holding it back; striking from the bolt makes Foley's timing right whichever mod's mixin runs first. One thunder bed, all round the listener, mixes every flash still rumbling (at most 6) and stops 25 s after the last. The air's temperature comes from the biome and its humidity from the rain. Shut in, it is low-passed down to 200 Hz, using the wind's exposure to the sky.
- The client gametest summons a bolt 40 blocks off and checks the thunder plays and the game's is replaced.
- **Cost:** about 4 ms to make each flash's thunder, once, on the stream thread.

## M4 results (2026-10-12)

- **Layered, not replaced:** when the game plays a block's hit (mining), place, fall (landing) or break sound, Foley plays its own with it. The material comes from the block where it's still there (the one hit, placed or landed on), from its tags (ice, plants, dirt, sand, wool and snow), else from its `SoundType`, which the game already picks by material. Sound types are told apart by name, read once by reflection (26.3 is unobfuscated).
- **Materials** (`Material`, 9 of them) have measured mechanical constants: Young's modulus, density, Poisson's ratio. Their loss factors are raised for blocks held by their neighbours. The vibrating structure is a plate:
  - a 25 cm stone slab;
  - a floor of 2.5 cm boards;
  - 5 cm of steel;
  - a 1 cm glass pane.
  For each, the simply supported plate's modes are f = (π/2)·h·√(E/12ρ(1−ν²))·((m/a)²+(n/b)²). Steel's first mode is 240 Hz, ringing for seconds; stone's is about 1 kHz, gone in a tenth of a second.
- **A strike:** each mode is weighted by:
  - how the strike point excites it (sin(mπx₀/a)·sin(nπy₀/b));
  - the radiated acceleration (∝ f);
  - the spectrum of the Hertz contact pulse;
  - radiation efficiency √(f/(f + f_c)), with f_c = c²/1.8hc_L the coincidence frequency.
  The 48 strongest under 16 kHz are kept, slightly mistuned. The contact time is Hertz's τ = 2.87·(m²/(R·E*²·v))^⅕. A steel tool on rock lasts a fraction of a millisecond (bright); on soil, several; a body landing on shoe soles, tens (dull). Gravel adds grain clicks, earth a few dull ones, plants a rustle.
- **A break:**
  - **Brittle (glass, ice, stone):** a crack, then 16 to 40 fragments sized by N(>L) ∝ L⁻². Each rings at a free plate's 0.453·c_L·t/L², with overtones at 1.6, 2.3 and 3.1 times that. Each falls 0.2 to 0.8 m and bounces with restitution e: √e as loud and e as long each time, with a click at every landing. Lying on the ground, a shard is damped at least 25 a second.
  - **Wood:** crackling fibres give way (sizes ∝ E^−1.7).
  - **Others:** earth and gravel crumble in grains; plants snap and rustle; metal and soft blocks give a dull blow.
- **Delivery:** 4 contacts × 9 materials × 8 variants are generated one-shots (`foley:<contact>_<material>`). Each is made once, the first time it plays, on the game's background executor, as the game loads files. A glass shatter takes 27 ms to make, a tap well under 1 ms (bed_bench).
- The client gametest plays the game's glass breaking and a fall on stone, and checks both got Foley's layer.

## M5 results, part 1: ambient life (2026-10-12)

- **Crickets:** a field cricket's chirp is 3 or 4 syllables, one for each closing stroke of the wings, 15 ms long and 35 ms apart. Each is a nearly pure tone near 4.5 kHz from the harp, rising 40 Hz per °C and gliding down 3 % as the stroke slows. Chirps come at Dolbear's rate: T°F = 50 + (N − 40)/4, so N ≈ 7.2·T°C − 32, or 112 a minute at 20 °C. Below about 8 °C there are none. (The plan's first draft had the formula wrong.)
- **Birdsong:** the syrinx's labia are a self-sustained van der Pol oscillator, x″ = μω(p − x²)x′ − ω²x, after Mindlin and Laje. The air sac's pressure p sets them going, and below threshold they fall silent. The syringeal muscles' tension sets ω, following each syllable's pitch contour. The integrator is semi-implicit Euler with ω pre-warped so it rings at the intended pitch. A tracking band-pass stands in for the trachea and beak, which songbirds tune to the note. Each of 12 species is made up from its number: 2 to 4 kinds of syllable (whistles, sweeps, trills) between 1.5 and 8 kHz, sung in a motif with repeats. Each rendition varies a little, and some are cut short.
- **Frogs:** glottal pulses, 60 to 150 a second, ring the vocal sac as a resonance. A 3 cm tree frog's is about 2.5 kHz and a 15 cm bullfrog's 500 Hz, with a weaker one at 2.3 times that. Calls come in 1 to 3 notes.
- **In the game** (`Life`), only in the Overworld, and only outside enough (the wind's exposure over 0.2):
  - **Birds:** sing from a random leafy treetop within 24 blocks, about one song every 4 s by day, three times as many at dawn, none in the rain. Each biome draws from its own three species.
  - **Crickets:** up to 4, each settled on a grass block within 14 blocks, chirp on and on at night from 10 °C (the biome's temperature, 5 °C cooler at night).
  - **Frogs:** call from still water at night or in the rain, often in swamps, where they're bigger.
  Each call is a generated one-shot played where the animal is, so it pans and Earshot shapes it. Variants: 12 species × 4 songs, crickets at 5 temperatures × 4, frogs at 3 sizes × 4.
- The client gametest sets midnight on a superflat and checks crickets chirp.

## M5 results, part 2: footsteps (2026-10-12)

- **Footsteps** are a fifth contact, layered under the game's step sounds (for players and mobs alike). Each is a heel strike: about 8 kg of foot and shank on a 5 cm heel of shoe sole, at 1 m/s. Hertz gives tens of milliseconds of contact, so a step is a dull thud through the block's modes. Grains crushed go with the blow's momentum (√(m·v / 350 kg·m/s), against a landing's), so gravel and sand crunch under a step and more under a landing.
- **Snow** is now its own material: settled snow, a few MPa and a third the density of water. It crunches and squeaks as the bonds between its grains break: crackling bursts between 1.5 and 4 kHz with sizes ∝ E^−1.7, over the 0.1 s the foot takes to sink in. In snow, that is most of what's heard. A thud's low-pass now applies to the click alone, not the grains.
- Variants: 5 contacts × 10 materials × 8. The client gametest also plays the game's snow step and checks it got Foley's layer.

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
