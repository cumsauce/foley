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

## M5 results, part 3: settings, and the first beta (2026-10-12)

- **Settings** (`config/foley/foley.json`, in Options → Music & Sounds → Foley…, and Mod Menu):
  - Foley's volume (0 to 100 %, under the game's own sliders; 0 is off);
  - for each sound it replaces (rain, thunder, water, drips, fire, lava), "Foley" or "The Game's";
  - for each it adds (wind, strikes and breaks, footsteps, birds, crickets, frogs), on or off.
  A missing or bad value falls back to its default.
- **Bypass key** (unbound until set): switches Foley off and on at once for A/B listening. Off, every replaced sound is the game's again and everything Foley adds stops.
- The client gametest opens the settings from Music & Sounds (and screenshots them) and checks that, bypassed, Foley leaves the game's thunder alone. A unit test covers the settings file.
- Released as **0.1.0-beta.1**.

## M6 results: procedural mob voices (2026-10-12)

- **A source–filter voice:**
  - **Source:** Rosenberg glottal pulses, opening over 40 % of the period and closing over 16 %. They have cycle-to-cycle jitter and shimmer, roughness (every other pulse weaker: the period doubling of a growl or groan), and breath noise. The lips radiate the flow's change.
  - **Tract:** a Klatt cascade of four formant resonators at F_n = (2n − 1)·c/4L for a tract L long, times each vowel's factors ([a], [e], [i], [o], [u], from a human tract's measured formants), with bandwidths a tenth of each. Mouth shut, a nasal murmur near 250 Hz is heard instead.
  - **Size:** tract length sets formant spacing (ΔF = c/2L), and both follow body size, as Fitch found: a cow's 45 cm tract against a chicken's 7.
- **Calls**, made up afresh each time, mob by mob:
  - a cow's "mm-ooo";
  - a pig's rough, nasal grunts;
  - a sheep's "baa" with a bleat's tremolo;
  - a hen's clucks and "bawk";
  - a villager's "hmm", "hrm?" or "hm.", mouth shut;
  - a zombie's rough, breathy groan;
  - a wolf's panting or bark;
  - a cat's "mi-a-ow".
  Every mob also has a hurt call (higher, rough, sudden) and a death call (falling, fading, breathier).
- **In the game** (`Voices`): off by default, switched on mob by mob in the settings. The game's ambient, hurt and death sounds are replaced in all their variants (`entity.wolf_big.hurt`, `entity.cat.ambient_baby`, a zombie villager's). Foley keeps the game's volume and pitch through an accessor mixin, as a sound's getters need it resolved. So babies are higher, and bigger and smaller variants a little lower or higher. The game's subtitles are kept.
- The client gametest checks a sheep isn't voiced by default, and is once the sheep is switched to Foley's. The settings test covers the per-mob switches.
- Released as **0.1.0-beta.2**.

## Milestones

0. **M0 Groundwork.** Repo, Rust workspace, FFM bridge, CI and natives packaging, all copied from Earshot. Spike: one procedural `FabricSoundInstance` stream (a sine, then filtered noise), and one generated buffer through the `SoundBufferLibrary` mixin. Measure CPU on the M3. **Exit:** both paths play in game, and Earshot shapes them.
1. **M1 Rain and wind beds.** Marshall–Palmer drops on sampled surfaces with the Minnaert water model; turbulence wind with exposure. **Exit:** standing under a tree, on stone and by a lake in the rain sound clearly different; wind rises on a mountain top.
2. **M2 Water, drips, lava, fire.** Bubble populations, drips, viscous lava, combustion with crackles.
3. **M3 Thunder.** The Ribner–Roy channel model, replacing vanilla thunder (Earshot's thunder delay still applies).
4. **M4 Modal impacts and breaking.** Material modal tables, block breaking and landing, glass and stone fracture.
5. **M5 Ambient life, footsteps, settings, first prerelease.**
6. **M6 Procedural mob voices (optional, off by default).** Vocal-tract models for the animals and monsters: source-filter synthesis, with formants and pitch from each mob's size and kind (a cow's low moo, a chicken's cluck, a villager's "hmm"). Each mob can be switched between Foley's voice and vanilla's.

7. **M7 Voice engine 2, and the night's four.** A new voice engine (below) and the most-heard mobs on it.
   - **Remade:** the eight from M6 (cow, pig, sheep, chicken, villager, zombie, wolf, cat).
   - **The hostiles everyone meets every night, with their variants:**
     - zombies (husk, drowned, zombie villager);
     - skeletons (stray, bogged): a dry rasp over rattling bones, which are M4's modal impacts;
     - spiders (cave spider): stridulation chitter and hiss;
     - creepers: the fuse hiss, as turbulent flow through a narrow opening.
   - **Bats:** squeaks and FM chirps, the most common sound underground.
8. **M8 Everyday animals.**
   - horse, donkey and mule (whinny; a donkey's bray, alternating inhaled and exhaled voicing);
   - goat (bleat, and its scream);
   - llama and trader llama (hum);
   - rabbit, fox (screech), parrot (its imitations drawn from Foley's voices);
   - bee (a wingbeat buzz near 230 Hz);
   - polar bear (growl);
   - panda, frog, camel, armadillo, turtle, ocelot;
   - the rest of the villager (yes, no, trade, celebrate), the wandering trader, and iron and snow golems (stone and metal creaks from M4's modes).
9. **M9 Rarer Overworld hostiles.**
   - enderman: inhaled, low, babbling pseudo-speech, and its stare cry;
   - witch: a cackle, from a laughter model of rhythmic bursts with gasps;
   - slime: wet squelches;
   - phantom: a screech with deterministic chaos;
   - the illagers (pillager, vindicator, evoker): muttered pseudo-speech, scoffs, a darker "hmm";
   - ravager: a roar from a long tract;
   - vex: chirps;
   - guardian and elder guardian: underwater moans;
   - silverfish and endermite: chitter.
10. **M10 The Nether and the End.**
    - piglins and brutes: snorting grunts and mutters;
    - zombified piglin;
    - hoglin and zoglin: big, rough roars;
    - ghast: a wail with sobs and inhaled gasps, never a scream;
    - blaze: fire-breath roar and its rods' clatter;
    - magma cube, strider (warbles), wither skeleton, shulker;
    - the ender dragon and the wither.
11. **M11 Rare and special.**
    - warden: heartbeat, sniffing, a deep roar;
    - breeze: aeolian whistles from M1;
    - creaking: stick-slip wood creaks;
    - allay: hums;
    - sniffer, dolphin (whistles and clicks), axolotl, glow squid, tadpole, and fish flopping (M4 impacts).

Each of M7–M11 is one release, ordered by how often players hear the mobs (spawn weights, where they live, how loud they are), so each release covers as much as it can. Every mob stays switchable between Foley's voice and the game's.

## M8 and M9 results: everyday animals and rarer Overworld hostiles (2026-10-12)

- **The engine grew:**
  - **Biphonation:** a second, independent glottal source at a non-harmonic ratio, for horses, whose whinnies have two fundamentals (Briefer et al. 2015).
  - **Builders:** methods for writing calls compactly.
  - **Pseudo-speech:** consonant–vowel syllables with phrase intonation and stresses, the consonants made in the tract (closures released, nasals, fricatives, breaths), with pauses between "words".
  - **Laughter:** bursts of breath and voice about five a second, falling, with an in-breath gasp.
  - **Sniffing and snorting:** unvoiced nasal breaths, with nostril flutter for horses.
- **Everyday animals:**
  - horses, donkeys and mules: whinnies, a bray alternating inhaled "hee" and exhaled, rough "haw", and snorts;
  - goats: a nasal, trembling bleat, and the screaming goat's rough one;
  - llamas: a hum with the mouth shut, and a staccato alarm call;
  - rabbits and armadillos snuffle;
  - foxes: gekkering (rapid rough clicks of voice), barks, a vixen's scream, sniffing, and sleep breathing;
  - parrots squawk, and imitate mobs in Foley's voices for them, pitched up;
  - bees: a wingbeat buzz at about 230 Hz (255 angry), harmonic-rich and wavering. One second loops seamlessly (whole wingbeats, whole wavers, crossfaded), so the game's own looping instance is pointed at it rather than replaced, and keeps following the bee and rising in pitch as it flies;
  - polar bears huff, cubs bleat, and adults give a growl as a warning;
  - pandas bleat, whimper when worried, bark when aggressive, and sneeze;
  - camels give a gurgling groan (sidebands at about 22 Hz, through the soft palate's pouch);
  - turtles hiss, and ocelots chirp;
  - villagers and wandering traders: "mm-hm!", "mm-mm", "hmm?" and a cheerful laugh. The trader is older and breathier;
  - iron golems ring with M4's metal and stone modes, and their joints creak by stick–slip; dying, they fall apart. Snow golems creak, crunch and collapse as snow.
- **Rarer hostiles:**
  - endermen: pseudo-speech on the in-breath, low, through a tract a little too long for a human voice, creaking at the end of a phrase; an inhaled gasp rising when stared at, and an inhaled, rough cry when angry;
  - witches cackle ("hee-hee-hee", pressed and falling, with a gasp) or mutter;
  - illagers (pillager, vindicator, evoker, illusioner) grumble in nasal pseudo-speech under their breath, and cheer at a raid;
  - phantoms screech, high and chaotic; their swoop is air rushing past, and their wings beat softly;
  - ravagers huff and grunt, roar, and groan when stunned;
  - vexes twitter, rising when they charge;
  - guardians moan like whales, gliding, with a few bubbles. Elder guardians are deeper and longer. On land they gasp and flop with wet slaps;
  - slimes: a gel body's low modes, heavily damped, under the sticky crackle of its surface, for squishing, jumping, attacking, hurt and death; small ones are higher;
  - silverfish and endermites chitter like small spiders.
- **One list, generated tables:** `scripts/gen_voices.py` holds the game's mobs and calls and Foley's for each. It checks them against the native enums, and writes `VoiceTable.java` and sounds.json's voice entries, with the game's own subtitles. That makes 52 native mobs, 43 calls and 215 voice events. The settings have 36 groups, each a bit of a 64-bit mask, with a button to voice them all (or none).
- **Checked:** spectrograms of the M8 and M9 calls (`--example spectrogram -- out.png m8|m9`) showed the guardians' moan buried under bubble clicks after an underwater filter. Guardians now call unfiltered (Earshot muffles them if the listener is under water). Slime bubbles are quieter.
- **Tests:** the bee loop's seam is no bigger than a step within it; a witch's cackle comes in at least four bursts; every mob's every call renders, bounded and finite. The gametest checks a horse, a screaming goat and a witch's cackle are Foley's, and that a bee's buzz is redirected.
- Released together as **0.1.0-beta.4**.

## M7 results: voice engine 2, and the night's four (2026-10-12)

- **The engine is built as planned:**
  - **LF source with Rd:** Fant's regressions give Ra, Rk and Rg, and α and ε are solved so the net flow is zero. Tables cover Rd 0.3 to 2.7.
  - **Nonlinear phenomena:** jitter, shimmer, subharmonics, sidebands, chaos with pitch jumps, and creak.
  - **Ingressive voicing:** each pulse is played time-reversed, breathier.
  - **Tract:** a Kelly–Lochbaum waveguide in pressure waves, 3.6 mm sections at two steps a sample, with a three-way velum junction to a nasal tube, wall losses, and frication injected at constrictions. The tongue follows Pink Trombone's model.
- **Measured:**
  - A uniform 17 cm tube resonates at c/4L (504 Hz), and a 45 cm one at 191 Hz.
  - The vowels were tuned against Peterson and Barney's men's formants, and a test holds each within 20 % ([i] 300/2310 Hz, [ɛ] 510/1820, [æ] 640/1690, [ʌ] 730/1220, [ɑ] 720/1110). My first shapes (open-mouthed back vowels) gave near-uniform tubes, which a spectrogram showed.
  - The first breath noise also swamped the harmonics above 2 kHz; it now sits about 10 dB down and is darker.
- **Mobs:**
  - The eight from M6 are remade on the new engine.
  - Zombie kin (husk: dry and rasping; drowned: gurgling in air, muffled with bubbles under water; zombie villager: nasal).
  - Skeletons are free-bar bone modes (E = 17 GPa, ρ = 1900 kg/m³) as teeth chatter and knocks, with sometimes a faint dry breath. Death is every bone falling and bouncing. Strays add rime ticks; bogged ones are damp and drip.
  - Spiders are stridulation (a file of teeth at 300–700 a second ringing chitin), mandible ticks and hissing; cave spiders are smaller and higher.
  - The creeper's fuse is a turbulent hiss that swells over 1.5 s, fluttering, with sparks and a faint rising whistle. Hurt, its leafy body crunches; dying, it deflates.
  - Bats are social squeaks sweeping down from about 15 kHz, and wing flutter.
  - Wolves gain growls (rumbling out and in), pants and whines. Cats gain purrs (about 25 pulses a second on both breaths), hisses and begging meows.
- **Zombies close in** (asked for 2026-10-12):
  - Within 8 blocks, a zombie's ambient call is its strained one: pressed harder, rougher, louder, breathing hard after.
  - Further, it's a little quieter.
  - Within 10 blocks, each of the four nearest breathes, the sound following it. At the edge, steady breaths every 3 s or so; within 4.5 blocks, heavy, ragged and voiced, about every second. It gets louder the closer the zombie is. A drowned under water doesn't breathe.
- **Settings:** the voice switches are now groups. Zombies cover husks, drowned and zombie villagers; skeletons cover strays and bogged. New groups: spiders, creepers and bats.
- **Cost:** each call variant is made once, off-thread: a zombie groan in 10 ms, a cow's moo in 25 ms, a creeper's fuse in 1 ms.
- **Checks:** the gametest switches zombies and creepers on, summons a zombie three blocks away and checks it breathes, and checks a creeper's fuse is Foley's. `voice_samples` and `spectrogram` render the calls for listening and looking.

## Mob voices, M7 onward: the voice engine and the brief (2026-10-12)

**The brief:**
- Realistic above all.
- Hostile mobs unsettling, but not outright horror: no shrieks, jump scares or sudden loud onsets on ambient calls, and never louder than the game's.
- Human voices where the mob is human-like; animal ones where it isn't. Mobs with no voice (skeletons, creepers, spiders) sound like what they are made of.

**What the research says makes a voice real, and what makes it unsettling:**

- **Nonlinear vocal phenomena** are subharmonics, sidebands, deterministic chaos and pitch jumps. Anikin's soundgen work shows they can be added parametrically to synthetic human and animal calls that listeners take for real. Chaos reads as aversive (fear, pain). Subharmonics read as dominant, and harsh voices as large. The effect is strongest in ambiguous sounds such as moans and gasps (Anikin 2020; Anikin, Pisanski and Reby 2021). Nonlinear phenomena also make synthetic animal calls alarming to human listeners (Massenet et al. 2022; Terrade et al. 2025). This is the main lever: real voices do this when distressed, so used in measure it is unsettling without being fake.
- **Roughness:** amplitude modulation at 30–150 Hz is what screams and alarms share, and it engages the amygdala (Arnal et al. 2015). For hostiles it is used sparingly and at low depth, so it unsettles without alarming.
- **Nonlinearity in film:** horror soundtracks are dense with nonlinear elements, while dramas use abrupt frequency changes (Blumstein, Davitian and Kaye 2010). Foley stays at the drama end.
- **Size from formant spacing:** formants follow vocal-tract length, and listeners read body size from formant spacing and pitch, as M6 already uses (Fitch 1997). A mismatch between a human-like voice and too-long a tract reads as "something off". Used for the enderman.
- **Quality and speed:** recent lightweight vocoders show that a parameter-free DSP source–filter vocoder (periodic plus noise excitation, F0 and periodicity, a vocal-tract filter) can match neural vocoders at a tiny fraction of the cost (Meta's ultra-lightweight DDSP vocoder, 2024; articulatory DDSP, 2024). Foley needs no neural network: its control comes from the mob's behaviour and anatomy, not a learnt model. That keeps it deterministic and cheap.
- **Nonverbal vocalizations:** current TTS research (NonverbalTTS 2025, NVBench 2026) treats breaths, sighs, groans, grunts and laughs as the hard, expressive part of a voice. Foley's mobs are almost entirely made of these, so the engine is built around them rather than words.

**The engine:**

- **Source:** the Liljencrants–Fant glottal flow model, set by the single Rd parameter (Fant 1995), which runs from pressed and creaky through modal to breathy. Aspiration noise is modulated by the glottal flow. On top come the nonlinear phenomena above: subharmonics (every *n*th pulse), sidebands (amplitude modulation), chaos (bursts of strong jitter and shimmer) and pitch jumps. Creak (vocal fry) is irregular pulses at 20–70 Hz.
- **Ingressive phonation:** voicing on the in-breath, as a donkey brays and as a groan catches. Its spectrum differs (noisier, higher), and it is quietly unsettling in a human-like voice.
- **Tract:** a Kelly–Lochbaum digital waveguide of about 40 sections, with an area function for each target (vowels, closures, a tube for animals). Transitions between targets give coarticulation for free. It has a nasal branch, with frication noise injected where the tract narrows and the flow is high. One waveguide costs a few million operations per second of sound. Calls are rendered off-thread as generated one-shots, as now, so nothing runs per frame.
- **Expressivity:** each call is made from arousal and valence, which set its pitch, effort (Rd), loudness and how many nonlinear phenomena it has. Ambient calls sit low, hurt calls are high and short, and death calls fall and fade. More variants per call (16 to 24) avoid repetition. Hostiles get more chaos, subharmonics and creak, breath, and ingressive gasps, held below the level where it turns to horror.

**Sources:**
- Anikin, *Soundgen*, 2019: https://cran.r-universe.dev/soundgen
- Anikin, *The perceptual effects of manipulating nonlinear phenomena in synthetic nonverbal vocalizations*, Bioacoustics 2020: https://bioacoustics.info/article/perceptual-effects-manipulating-nonlinear-phenomena-synthetic-nonverbal-vocalizations
- Anikin, Pisanski and Reby, *Harsh is large*, 2021: https://comm.ucla.edu/wp-content/uploads/2026/01/Anikin-et-al.-2021-Harsh-is-large-nonlinear-vocal-phenomena-lower-vo.pdf
- Massenet et al., *Nonlinear vocal phenomena affect human perceptions*, 2022: https://comm.ucla.edu/wp-content/uploads/2026/01/Massenet-et-al.-2022-Nonlinear-vocal-phenomena-affect-human-perceptions.pdf
- Terrade et al., *Nonlinear phenomena make animal calls alarming for human listeners*, 2025: https://comm.ucla.edu/wp-content/uploads/2026/01/Terrade-et-al.-2025-Nonlinear-phenomena-make-animal-calls-alarming-for-human-listeners.pdf
- Arnal et al., *Human screams occupy a privileged niche*, 2015: https://pmc.ncbi.nlm.nih.gov/articles/PMC4562283
- Blumstein, Davitian and Kaye, film soundtracks and nonlinearity, 2010: https://www.sciencenews.org/article/chaos-makes-scream-seem-real
- *Ultra-lightweight neural differential DSP vocoder*, 2024: https://www.alphaxiv.org/abs/2401.10460
- *Articulatory synthesis using differentiable DSP*, 2024: https://www.researchgate.net/publication/383753320_Fast_High-Quality_and_Parameter-Efficient_Articulatory_Synthesis_using_Differentiable_DSP
- *Four decades of digital waveguides*, 2026: https://arxiv.org/pdf/2604.12878
- NonverbalTTS, 2025: https://arxiv.org/html/2507.13155
- NVBench, 2026: https://arxiv.org/html/2604.16211v2

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
