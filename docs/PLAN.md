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
- **Birdsong:** the syrinx's labia are a self-sustained van der Pol oscillator, x″ = μω(p − x²)x′ − ω²x, after Mindlin and Laje. The air sac's pressure p sets them going, and below threshold they fall silent. The syringeal muscles' tension sets ω, following each syllable's pitch contour. The integrator is semi-implicit Euler with ω pre-warped so it rings at the intended pitch. A tracking band-pass stands in for the trachea and beak, which songbirds tune to the note. Each of 57 species (grown from 9 in M5 to 57 in M13) is made up from its number: 2 to 4 kinds of syllable (whistles, sweeps, trills) between 1.5 and 8 kHz, sung in a motif with repeats. Each rendition varies a little, and some are cut short.
- **Frogs:** glottal pulses, 60 to 150 a second, ring the vocal sac as a resonance. A 3 cm tree frog's is about 2.5 kHz and a 15 cm bullfrog's 500 Hz, with a weaker one at 2.3 times that. Calls come in 1 to 3 notes.
- **In the game** (`Life`), only in the Overworld, and only outside enough (the wind's exposure over 0.2):
  - **Birds:** sing from a random leafy treetop within 24 blocks, about one song every 4 s by day, three times as many at dawn, none in the rain. Each biome draws from its own three species.
  - **Crickets:** up to 4, each settled on a grass block within 14 blocks, chirp on and on at night from 10 °C (the biome's temperature, 5 °C cooler at night).
  - **Frogs:** call from still water at night or in the rain, often in swamps, where they're bigger.
  Each call is a generated one-shot played where the animal is, so it pans and Earshot shapes it. Variants: 57 species × 4 songs, crickets at 5 temperatures × 4, frogs at 3 sizes × 4.
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

## F0 results: the integration API (2026-10-15)

Other mods can now use Foley's synthesis. The structural pieces are laid so the rest (materials, primitives, voices) only ever adds to them:

- **A separate `foley-api` artifact** (`mod/api`, mod id `foley-api`): the stable types other mods compile against — `Foley` (a static facade), `FoleyEntrypoint`, `FoleyApi`, `FoleySounds`, and `SoundModel`, the whole catalogue as a sealed interface of records with the parameters each model takes, checked where they are built. The main mod implements it and bundles it (JiJ); Loom's `include` nests only through remapJar, which the unobfuscated setup doesn't run, so the jar task nests it directly. The API is found through a ServiceLoader provider, so the jar needs no implementation classes and a missing Foley simply reports `Foley.available()` false.
- **A `foley` entrypoint** other mods declare, called one by one once the synthesis and settings are up; a failing addon is logged with its mod id and skipped. Registration closes when the last returns, and each registration names its addon: a sound's namespace is its owner's.
- **The same sound in code or in a pack**: `SoundCodecs` reads and writes every model from one `"model"` field (a hand-rolled dispatch, so the JSON shape is exactly `{"model": "drip_into_water", "radius_mm": 1.2}`); `assets/<ns>/foley/sounds.json` is read at every resource reload, its errors logged per entry, and a code definition wins over a pack's for the same id.
- **Delivery is M0's, generalized past `foley:`**: a registered sound's variants live at `<ns>:foley/<name>_<n>`, served by the `SoundBufferLibrary` mixin (rendered off-thread, cached as any buffer) and kept through the game's sounds.json file check by shape. Playing needs no sounds.json of the addon's own: the played instance resolves itself from the registry. Playback follows Foley's volume and bypass and shares its six-one-shots-a-tick share.
- **Checked in the real game**: the gametest mod is the reference addon — two sounds from its entrypoint, two from its pack — all four render and play, and registering after the entrypoint throws. Unit tests cover every model's round-trip and the registry's rules.
- **Not yet**: publishing the api jar to the Modrinth maven (its coordinates are left as a TODO in docs/INTEGRATING.md); materials, primitives and voices are F1–F3.

## F1 results: custom materials (2026-10-15)

Other mods' blocks can now have their own physics for Foley's strikes, breaks and footsteps.

- **The physics became a value.** `Material::props()`'s private `Props` is now the public `MaterialProps`, with a `behavior` field naming the built-in whose habits it keeps (how it grains, scuffs, crunches, cracks — the branches that read `material` now read `props.behavior`). `contact_with` and `fracture_with` render from one; the old enum entry points delegate. ABI 15 adds `foley_render_contact_custom(props, count, kind, speed, seed, …)`, nine floats (behaviour index, E, ρ, ν, η, plate thickness, fragments, restitution, shard) in documented ranges, checked like every argument.
- **The Java API grew `FoleyMaterials`** (and `MaterialDescriptor` in the api artifact, with its own codec): register a material, map blocks by exact block, tag or `SoundType`. `Layers` asks the custom registry first — by block where the block can be asked (tap, place, land, step), by sound group for a break, whose block is already gone. Custom layers are named `foley:generated/custom_<index>_<contact>` (8 variants), rendered through the same generated-buffer path, and play through a self-resolving instance, needing no sounds.json.
- **Packs**: `assets/<ns>/foley/materials.json` — `materials` (name to descriptor), `blocks` and `tags` (block or tag id to material name), read at each resource reload; code definitions win for the same name, and the pack's mappings replace the last reload's.
- **Checked in the real game**: the gametest's own material ("crystal": glassy physics, Young's modulus 90 GPa) has the amethyst sound type and block mapped onto it; the game's amethyst break layers Foley's custom layer. Unit tests cover the descriptor's checks, its JSON round-trip (Young's modulus read in GPa, stored in Pa), the floats' order against the natives, and the registry's namespace rules.
- **In the engine**: a custom material keeps its own ring (the modes come from its physics) and borrows its behaviour's manners (stone's physics with snow's habits crunches on after the heel).

## F2 results: primitives and beds (2026-10-15)

Mods can now build sounds of their own from primitives, and run beds of Foley's streaming models under their own ids.

- **Four primitives** (`crates/foley-synth/src/primitives.rs`), each rendered to a one-shot, bounded and peak-normalized: a bubble's Minnaert ring (radius 0.05–20 mm, wavering with the seed); a burst of band-passed noise (centre 20–20000 Hz, Q 0.5–20, 5 ms–5 s, gain); a tone gliding to a target with light vibrato; a gust of three staggered surges whose pitch and level rise with the wind (level ∝ U³, against 20 m/s). ABI 16 adds `foley_render_primitive(kind, params, count, seed, …)`, ranges checked like every argument; a test renders each, checks it ends in silence, and confirms same seed → same sound.
- **The API grew**: `SoundModel` gains `Bubble`, `NoiseBurst`, `Tone` and `Gust` (with codecs, checked where built, named `bubble`/`noise`/`tone`/`gust` in packs), rendered through the same generated-buffer path as the catalogue.
- **Beds**: `FoleyBeds` — an addon starts Foley's rain, wind, water, fire, lava or thunder under its own id, tells its scene (which glides, as Foley's own do), places it in the world, weighs it, and retires it. Unlike the API's registrations, beds come and go whenever the addon likes. **At most 2 run at once**: the game's 8 streams are nearly spent between the game's music and Foley's own beds. `BedSound` gained a self-resolving mode (an addon's bed has no sounds.json entry: it resolves itself to a streaming sound), a retire flag (the game retires it on its next tick), and the bypass now silences addons' beds too.
- **Checked in the real game**: the gametest plays a tone primitive as a registered sound; starts a water bed and a second (a third is refused), hears it render, retires both. Unit tests cover the primitives' codecs and ranges.
- **Deferred deliberately**: render-level mixing (several primitives under one sound name) — composition stays at playback level until an addon asks for more.

## F3 results: creatures of other mods (2026-10-15)

Mods can now have their creatures voiced by Foley's synthesis.

- **The voice became a spec** (`crates/foley-synth/src/voice/creature.rs`): a `CreatureSpec` — tract length, usual pitch, and ceilings for roughness and breath — and five archetypes any creature needs (ambient, hurt, death, growl, breath), each written from the same `Seg` vocabulary as Foley's own calls with the spec scaling the nonlinear phenomena: rough sets chaos, subharmonics, sideband depth (30–150 Hz, the roughness of alarm) and creak; breathy sets Rd, breath noise and frication; the brief holds — no shrieks, nothing louder than the game's. ABI 17 adds `foley_render_creature(spec, call, seed, …)`, four floats in checked ranges.
- **The API grew `FoleyVoices`** (and `VoiceDescriptor` with its codec): register an entity type with a call map (game call name → archetype), and `Voices` speaks for those entities' sound events — variant suffixes and `_baby` pitches handled as the game's own are, the game's volume and pitch kept, delivered as generated one-shots under `foley:generated/voice_custom_<index>_<archetype>`. Off until players switch the group on: the settings gained a 37th group, "Addons' Creatures" (the 64-bit mask had room).
- **Packs**: `assets/<ns>/foley/voices.json` — entity id to voice and call map, read at each resource reload, code registrations winning for the same entity.
- **Checked in the real game**: the gametest's dolphin (a 30 cm tract at 300 Hz, a little rough and breathy) is voiced once "Addons' Creatures" is on and not before. Unit tests cover the descriptor's checks and codec, the floats' order, and the archetype names.
- **Deliberate limits**: the archetypes are generic — a creature whose calls are recognisably *it* (not just its size and temper) is Foley's hand-built mobs' craft, not yet shareable; per-creature renditions (`Individuals`) are the vanilla mobs' and don't yet carry to addons.

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

12. **M12 Cave atmosphere.** A placed bed that gives being underground its sonic identity: a sparse multi-position drip bed with Earshot's reverb and a deep subsonic rumble that grows with depth and the mass of stone above, so deep caves feel enclosed rather than merely quiet. The existing sparse `Life.caves()` is removed in favour of the bed. **Exit:** standing 50 m below surface in a dripstone cave, distant drips are heard from multiple positions and a low rumble is felt; the bed fades as the player approaches the surface or steps outside; vanilla caves with no exposure hear no wind but keep the bed.

13. **M13 More birds.** The syrinx engine 2 and a catalogue grown from nine temperate North American species to ~35, with per-biome rosters modelled on the real bird calls that would live in each biome.
    - **Engine 2:** the syrinx model gains frequency modulation (warbles, bobolink's burble), rapid AM-modulated sweeps (trills), inhaled chips, rattles, croaks, hoots, and a `Phrase` builder so each species is described by a sequence of segments rather than hand-coded.
    - **Per biome, modelled on the real call:**
      - taiga, snowy taiga: boreal owl, great grey owl, common raven, spruce grouse (drumming, the existing woodpecker model), pine grosbeak, boreal chickadee, varied thrush;
      - plains, sunflower plains: western meadowlark, horned lark, dickcissel, bobolink;
      - forest, flower forest, birch, old-growth birch: wood thrush, hermit thrush, scarlet tanager, red-eyed vireo;
      - jungle: keel-billed toucan, Montezuma oropendola, clay-colored thrush, great tinamou, crested guan;
      - savanna: lilac-breasted roller, red-billed hornbill, Kori bustard, southern yellow-billed hornbill;
      - badlands, mesa, eroded badlands, wooded badlands: cactus wren, curve-billed thrasher, greater roadrunner, pyrrhuloxia;
      - desert, desert hills, desert lakes: greater roadrunner, cactus wren, Gila woodpecker, verdin;
      - snowy tundra, ice spikes, snowy beach: snowy owl, snow bunting, willow ptarmigan;
      - cherry grove: Japanese bush warbler (uguisu), Japanese white-eye (mejiro), varied tit, Eurasian tree sparrow;
      - mangrove swamp, swamp: red-winged blackbird, common yellowthroat, prothonotary warbler, limpkin;
      - mountain (jagged peaks, stony peaks, meadow, grove): Clark's nutcracker, golden eagle, alpine chough, grey-crowned rosy finch;
      - beach, stony shore: common tern, sanderling (the gull stays);
      - mushroom fields: a fictional soft "moo-oo-oo" trill, since no real analog fits.
    - **Per-biome rosters:** `pickBird` is replaced by a biome-keyed roster of (species, weight) pairs, generated alongside `VoiceTable` so the catalogue is data, not code. **Exit:** a plains biome sounds like a plains biome; a savanna doesn't sound like a desert; a taiga doesn't sound like a forest; a mushroom biome has its soft trill.

M12 and M13 are environment, not mob voices, so they're ordered by what fills the most silent biomes: cave atmosphere (the Overworld's emptiest region) before the catalogue expansion.

14. **M14 Underwater ambience.** A listener state that takes over when the head is submerged: a bed of bubbles rising past the ear (rate ∝ depth × movement speed), a low pressure tone at the bottom of deep oceans, and a global low-pass on every Foley sound while the head is under. **Exit:** standing on the ocean floor, the world is muffled, bubbles rise past, deeper water pushes a low tone into the mix; emerging above the surface restores Foley's air within 1 s.

15. **M15 Nether ambience.** Wind is off in the Nether per M1, but nothing has replaced it; mob voices land in M10, and the dimension's identity comes from its environment. A handful of placed beds, weighted by sub-biome: a low hot ambient roar that never quite stops; basalt delta clatter (M4 modal impacts at a slow Poisson rate); soul sand valley's ash hiss (band-passed noise modulated by soul fire); warped forest and crimson groves' insect chorus (a bed of cicada-like drones at the warped tempo); a continuous lava-lake rumble that's louder than the Overworld's because lakes are denser. **Exit:** a basalt delta sounds like clatter; a soul sand valley sounds like ash; a warped forest has its chorus; lava lakes are felt everywhere.

16. **M16 End ambience.** Sparse and eerie: a low ambient drone (slowly modulated sub-100 Hz tone, the canonical "space" sound); a very low faint whisper of pseudo-speech far off (endermen whispering at the central pillar, played at low extents); the ender dragon's wings and roars as a placed bed at the central island (a subsonic wingbeat bed with the dragon's roar laid in at a slow Poisson rate). **Exit:** on the central island the whisper is around you; flying through the outer islands the drone is there alone; near the dragon's perch the wingbeats rise; killing the dragon silences the wingbeat and roar beds but not the drone.

17. **M17 Surf as a placed bed.** The `surf()` model exists and is fired as one-shots from `Life` every 6 to 12 s, so a stormy shore sounds discrete ("crash… crash… crash"). Turn it into a placed bed so the breaking surges are continuous, the texture is denser in a higher U, and storm surges are layered on top. Three height options × continuous = drop from a low lapping to a 3 m wave every couple of seconds; storms add an extra low roar. **Exit:** a stormy beach is continuous surf, not discrete crashes.

18. **M18 Aeolian tones wired to placement.** M1's wind bed has the Strouhal `0.2·U/d` model for fences, bars and leaves listed in its plan but never wired to objects in the world. M18 places aeolian sources at every iron bar, fence, lantern, bell and bamboo within 24 blocks of the ear, with the right Strouhal frequency for each, and adds a soft ring from bells and lanterns in proportion to the wind speed. Leaves rustling already exists in M1's turbulence; it gets tied to leaf-bearing blocks in the round rather than a generic exposure factor. **Exit:** standing next to a fence in the wind sings; a bell rings in a storm; a bamboo grove howls.

19. **M19 Biome-specific beds.** Some biomes still sound like every other biome. M19 adds the missing beds:
    - **Desert / badlands / desert hills / desert lakes:** dry sand hiss underfoot (granular bed, not one-shot footsteps), a hollow wind in the open plain, optional dust devils (small vortex rings, level ∝ U⁵, only when U > 8 m/s);
    - **Mushroom fields:** a low, strange ambient hum unique to the biome;
    - **Lush cave / dripstone cave:** the cave bed stays (M12), but each gets its own colour (lush: a faint, soft high-frequency "dripping" hiss from the moss; dripstone: more drips, less rumble);
    - **Deep dark:** silence is part of the design, but a very low subsonic hum and the scuttling of sensors as one-shots every 30 s or so;
    - **Snowy / ice spikes / frozen ocean:** a "hush" bed — high-frequency absorption (above 4 kHz cut by a slow LPF) layered into the wind bed only in cold biomes, so snow absorbs the highs;
    - **Mangrove swamp:** a humid insect drone — band-passed noise at 4 kHz, level proportional to mangrove leaf count within 24 blocks, layered under the frogs and crickets already there.
    **Exit:** each biome listed sounds distinct; the open-air biomes (plains, savanna, taiga) keep their M13 identity.

20. **M20 Village ambience.** Villagers have voices (M6/M7/M8) but no village-level ambience that makes a town feel alive from a distance: a distant murmur of voices and animal calls (lower passband, panned to the village bell); the village bell itself (one of the most recognisable vanilla sounds, not replaced or layered); smith sounds (anvil ring, hammer blows); scattered chicken / cow / sheep / wolf placed at the village's structures, not by mob spawning. The village's centre (the bell block) is its anchor. **Exit:** a village heard from 50 m away is recognisably a village: bell + murmur + a chicken here, an anvil there; the bell rings more often in a storm; the smith's anvil is heard across the village.

21. **M21 Active structures.** Sounds from objects that exist in the world that Foley doesn't touch:
    - **Minecarts on rails:** the rhythmic clatter (regular and powered) and the rush when passing. The wheel-impact modal model already exists from M4; M21 wires it to minecarts and renders per-rail-type one-shots;
    - **Doors:** wooden doors open and close with stick–slip hinge creaks (M7 already has the model); iron doors and trapdoors get metallic creaks (M4's metal modes);
    - **Chests / trapped chests / shulker boxes:** opening and closing with a small puff of air (the existing drips model with a different envelope); trapped chests get a faint redstone click;
    - **Redstone:** a faint mechanical click when a repeater or comparator changes state, especially in clock circuits;
    - **Boats:** water lapping at the hull (a small placed bed per boat) and creaking wood;
    - **Sculk sensors:** they vibrate; the surrounding sculk should ring softly (a triggered bed of low-Q tones played from the sensor and from nearby sculk blocks).
    **Exit:** a minecart system is heard before it's seen; an old wooden door creaks; a shulker clicks open; a redstone clock has a quiet rhythm; a sculk sensor "talks" to nearby sculk.

22. **M22 Subtle physical layers.** Density rather than new models:
    - **Settling after footsteps and block placement:** a tiny delayed tick (10 to 30 ms after the footfall's thud) for gravel shifts, sand settles, snow brushes;
    - **Mob brushes through plants:** only the player brushes today (the `Brush` class); sheep wading through tall grass, a spider on cobwebs, a chicken in a flower field;
    - **Peaceful-mob breathing:** only zombies breathe when close (M7); cows in a barn at night, sheep in a pen — very quiet, very warm;
    - **Ash and snow drifting:** almost inaudible but the right *texture* — high-frequency bandpassed noise at very low level during a blizzard or ash storm.
    **Exit:** the world feels lived-in: footsteps settle, mobs interact with their world, peaceful nights are warm, blizzards and ash storms have texture.

M14–M22 extend the environment work, ordered by what fills the largest remaining gaps: M14 is a listener state that applies globally underwater; M15/M17 fill the most-listened dimensions and the model-already-there-but-not-wired case (the small wins); M16/M18/M19 fill the biome gaps; M20/M21/M22 fill the world's quieter infrastructure.

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

## M12 plan: cave atmosphere

A placed bed for being underground. `Life.caves()` currently does only one thing — a 4 %/s settling stone or a 15 %/s drip in a random air cell 6–20 m away — so deep caves are mostly silent holes. The plan has long called for *"cave atmosphere layer over the game's own sounds and music"* (line 40), but the bed itself was never built.

**Approach.**

A new placed bed kind, `FOLEY_BED_CAVE`, sharing the placed-bed pattern of water/fire/lava. At most one running. Its scene sampler reads:

- **Depth:** blocks below surface, from the heightmap over the ear.
- **Stone mass above:** the count of solid blocks in a column from the ear up to the sky.
- **Openness to sky:** already computed for `WindScene`. Above ~0.2, the cave bed fades out over 2 s — outside sound takes over.
- **Drip rate:** derived from waterlogged blocks and stalactites in the column. A dry cave (no water, no dripstone) gets a quieter rumble only.

The bed itself is two layers mixed:

- **A multi-position drip bed.** A sparse Poisson process of drips at 3 to 5 randomly chosen ceiling positions within 24 blocks, each with a randomised phase, panned. Earshot's reverb colours the early tail; if Earshot is absent, a small built-in early-reflections model (comb-filter delays) stands in.
- **A sub-bass rumble.** Band-passed noise below 80 Hz, modulated in level by depth × stone mass. Deeper and deeper-stone means a more present rumble; in a shallow cave the rumble is below threshold and the drips alone are heard.

A small wrinkle: the existing `Life.caves()` and the bed shouldn't fight each other. `Life.caves()` is removed in M12 — its settling-stone and drip one-shots are folded into the bed (settling stones still trigger as one-shots at random, but only when the bed is running, and they replace the bed's drip layer momentarily to vary the texture).

**Settings.**

A new toggle, *Cave atmosphere*, alongside *Wind*, *Birds*, *Crickets*, *Frogs*. Off by default for players who already find caves too noisy with the reverb Earshot gives them.

**Exit criteria.**

- A player standing 50 m below surface in a dripstone cave hears drips from multiple positions and feels a low rumble; a plains cave 30 m down with no sky exposure has the drips and a quieter rumble; approaching the surface fades the bed out; opening the cave mouth (exposure > 0.2) silences it within 2 s.
- A surface cave with a wide opening sounds like outside: the bed is silent, the wind plays.
- The bed doesn't double-trigger: no two cave beds ever run at once.

**Tests.**

- Unit: the scene's depth/mass/exposure/drip-rate mappings match the spec; the bed cost stays under ~0.5 % of a core on the M3 VM.
- Client gametest: spawn at y = -50 in a dripstone cave, check the cave bed is playing and the second one is refused; spawn at y = -30 in a plains cave with no exposure, check it's playing; reach the surface, check it fades; open the cave mouth, check it's silent.

## M13 plan: more birds

Nine species and a weighted pick by biome category is enough for "any one biome doesn't sound like another" — but the catalogue is temperate North American, and `pickBird` lumps very different biomes together (jungle, swamp and mangrove are one bracket; badlands, savanna and plains share the default). M13 expands the syrinx model to capture the calls of real birds that live in each biome, and gives each biome its own roster rather than a weighted pick from a global list.

**Syrinx engine 2.**

The current model (M5) uses van der Pol for the labia and a band-pass filter as the tract proxy. The new species need calls it can't make today:

- **Frequency modulation.** A sinusoidal modulator on the labial frequency, depth and rate as parameters. The basis of every warble (wood thrush's middle phrase, the bobolink's burble).
- **Trills.** Rapid AM-modulated sweeps, ~8 to 12 Hz. The hermit thrush's slow opening spiral; the pine warbler's run.
- **Inhaled chips and rattles.** Very short broadband bursts (5 to 10 ms) at high pitch; a Poisson process of them is a rattle (snow bunting, varied thrush's second note).
- **Croaks and rasps.** Short, low-pitch pulses with strong breath noise (hornbills, guan, roadrunner's clatter).
- **Hoots.** Long pure-tone pulses with a slow decay (boreal owl, great grey owl, snowy owl).
- **Drumming at different rates.** The existing `woodpecker()` covers most woodpeckers, but a Gila woodpecker's roll is faster than a pileated's; spruce grouse drumming is even faster. A `drum(rate_hz, decay_s)` helper exposes it.

A new `Phrase` builder takes a list of segments of `[kind, parameters, duration]` and renders them in sequence. Each species is then a small handful of phrases picked by an RNG; the catalogue is descriptive rather than hand-coded. The 9 existing species are migrated to the new vocabulary and stay the same in spirit (M13 is engine + catalogue, not a rewrite of M5).

**Species additions (9 → ~35), modelled on the real call of the species that would live in each biome:**

- **Taiga, snowy taiga** — boreal owl (hoots, ~1 kHz, a "hoo-hoo-HOO-hoo" motif with a long first note), great grey owl (deeper, slower), common raven (a corvid croak, separate from the crow's caw), spruce grouse (drumming — the outdoor model with a faster rate), pine grosbeak (a soft three-note whistle with a rising third), boreal chickadee, varied thrush (single buzzy note, a pause, then a higher one).
- **Plains, sunflower plains** — western meadowlark (flutey, two phrases with a slow trill in the second), horned lark (high tinkles from a perch, ~6 kHz), dickcissel (a chipped "dick-dick-cissel"), bobolink (a bubbly rattle of overlapping sweeps, ~250 to 600 ms).
- **Forest, flower forest, birch, old-growth birch** — wood thrush (flutey spiral of three notes, the middle one trilled, the canonical "forest at dusk" call), hermit thrush (long slow spiral whistle, the most ethereal of the lot), scarlet tanager (a raspy "burry" call, like a robin with a sore throat), red-eyed vireo (short phrases strung with pauses; the vireo never seems to stop).
- **Jungle** — keel-billed toucan (a frog-like croak repeated), Montezuma oropendola (a liquid bubbling phrase that drops in pitch), clay-colored thrush (a slow deliberate whistled phrase), great tinamou (a long pure whistle at ~1.5 kHz, rising and falling), crested guan (a low rasping cluck).
- **Savanna** — lilac-breasted roller (a sharp rasping "krak-krak"), red-billed hornbill (a clucking series with a cackle), Kori bustard (a deep boom at ~250 Hz, very rare, only on warm mornings), southern yellow-billed hornbill (a reedy caw).
- **Badlands, mesa, eroded badlands, wooded badlands** — cactus wren (a rapid churring series, ~10 syllables/s), curve-billed thrasher (a rapid whistled phrase with two repeats), greater roadrunner (a low coo series with a bill clatter), pyrrhuloxia (a whistled "wheat-er-chew-chee-chee-chee").
- **Desert, desert hills, desert lakes** — greater roadrunner, cactus wren, Gila woodpecker (a rapid drumming roll, faster than the existing woodpecker), verdin (a high squeaky "see-see, hear-see").
- **Snowy tundra, ice spikes, snowy beach** — snowy owl (hoots, with a higher-pitched female version sometimes heard alongside the male's lower one), snow bunting (a short rattle), willow ptarmigan (a low croak and a brief rattle).
- **Cherry grove** — Japanese bush warbler (uguisu, a single prolonged "hooo" that wavers and then opens into a trill, the iconic cherry-blossom sound of Japan), Japanese white-eye (mejiro, a high rapid "chi-chi-chi" with a small whistle), varied tit (a fast "tsu-tsu-tsu"), Eurasian tree sparrow (a chirrup, more nasal than a house sparrow).
- **Mangrove swamp, swamp** — red-winged blackbird (the "conk-la-ree" — a conk then a trill), common yellowthroat (a rolling "witchety-witchety-witchety"), prothonotary warbler (a clear rising whistle), limpkin (a long mournful wail, ~1 kHz, slightly quavering).
- **Mountain (jagged peaks, stony peaks, meadow, grove)** — Clark's nutcracker (a "kraa-kraa-kraa" rattle), golden eagle (a high scream, rare, only above a height threshold), alpine chough (a high "chiff-chiff" in pairs), grey-crowned rosy finch (a soft trill).
- **Beach, stony shore** — herring gull stays; common tern wins (a high "kee-yah"); sanderling (a short low trill, only when waves are large).
- **Mushroom fields** — a fictional soft "moo-oo-oo" trill, half-bird-half-pipe-organ; no real analog fits, so invent something that suits the biome's strangeness.

A few cross-biome species stay global: the common raven reuses the corvid model in taiga; the house-sparrow chirrup is the Eurasian tree sparrow in cherry grove; the gull stays in beach but doesn't bleed into other biomes.

**Per-biome rosters.**

`pickBird` is replaced by a roster table generated by `gen_birds.py` (alongside `gen_voices.py`). Each biome maps to a list of `(species, weight)` pairs; the table is data, not code. Biomes without a roster (river, deep ocean, frozen river) stay silent for birds, as they should — waterbirds are a follow-up.

**Settings.**

- The existing *Birds* toggle stays; no per-biome switches. The rosters are correct by biome.
- A second toggle, *Rarer species*, gates the Kori bustard, golden eagle, alpine chough and the like, off by default — they're special and shouldn't crowd every visit to their biome.

**Exit criteria.**

- A plains biome at first light sounds like a plains biome (meadowlark, horned lark, dickcissel); a savanna doesn't sound like a desert (roller and hornbill instead of wren and roadrunner); a taiga doesn't sound like a forest (owls and varied thrush instead of wood thrush); a mushroom biome has its soft trill; a beach has the gull and tern; a coral reef stays silent.
- Birds only sing from the right biomes: no cactus wren in a forest, no wood thrush in a taiga.
- Engine 2 makes every existing call as well as the engine 1 version did: spectrograms of the nine originals are unchanged.

**Tests.**

- Unit: each new species renders, bounded and finite; same seed → same call; engine 2's renditions of the original nine match engine 1's within an audible tolerance (spectrogram correlation > 0.9 on each).
- Gametest: at noon in each biome with the right weather, at least one species from its roster is heard within 60 s; in a coral reef and a river, no birds are heard.

## M14 plan: underwater ambience

A listener state, not a placed bed. When the player's head is submerged, every Foley sound gets a global low-pass filter (cutoff at 1 kHz, the canonical underwater muffle) and Foley adds three things on top:

- **A bubble bed.** Bubbles rise past the camera at a rate ∝ depth × speed of movement, rendered from the existing `Bubble` primitive. Louder when starting from a still position — a sudden dive shakes them loose.
- **A pressure tone.** A very low sinusoid (~50 Hz) under threshold at depth 0, rising in level with depth down to ~30 at the ocean floor. Above sea level: silent. Below: present.
- **A muffled bed of distant water sounds.** The water/lava bed already plays under water per `PlacedBeds`, but the air-pocket sounds (bubble population, foam hiss) are dulled further.

Emerging above the surface fades the bubble bed and pressure tone over 1 s and removes the low-pass on Foley's other sounds immediately. The mob voices already do this kind of muffled underwater thing, so they stay as the test for the filter.

**Exit:** underwater the world is muffled, bubbles rise past, deep water pushes a low tone into the mix; emerging restores Foley's air within 1 s; surfacing while breathing resets cleanly (no double-fade).

**Tests:** a player at y = 55 in the ocean hears bubbles and the pressure tone rises with depth; emerging drops them within 1 s; Earshot's underwater filter and Foley's play together cleanly without double-muffling.

## M15 plan: Nether ambience

The Nether has wind off per M1; mob voices land in M10. The dimension's identity comes from its environment, and it's silent in the wrong way.

**Approach.** A handful of placed beds, weighted by sub-biome (the existing `Surroundings`/biome-aware sampler pattern):

- **Hot ambient roar:** a low, broadband bed below 200 Hz, level proportional to the closest lava lake's size. Always on in the Nether, fading in to the bed as the player approaches lava.
- **Basalt delta clatter:** M4 modal impacts on basalt tiles at a Poisson rate of ~1 a second per 4 basalt blocks. Limited to basalt deltas.
- **Soul sand valley ash hiss:** band-passed noise modulated by the height of the nearest soul fire block; the higher the fire, the louder and higher-pitched the hiss.
- **Warped forest / crimson grove chorus:** a bed of cicada-like drones at the warped tempo (fast, ~16 Hz, slightly off-pitch between individuals). Limited to warped forests and crimson groves.
- **Continuous lava-lake rumble:** the lava bed already exists; M15 keeps it on at higher gain than the Overworld, since lakes are denser.

The cave bed (M12) is silent in the Nether — Nether caves are open to the sky (sort of) and use the Nether's own beds instead. The skyless-but-open question is solved by an `isNether()` flag in `WindScene` and `CaveScene`.

**Exit:** a basalt delta sounds like clatter; a soul sand valley sounds like ash; a warped forest has its chorus; lava lakes are felt everywhere; nether fortresses have the clank of piglins and bastions (those are mob voices for M10).

**Tests:** spawn in each sub-biome, check the right bed plays; in a basalt delta the clatter is heard; in a warped forest the chorus is heard; cave bed is silent; spectrogram of the basalt delta clatter matches M4 modal impact on basalt.

## M16 plan: End ambience

Sparse and eerie. Three layers:

- **A low ambient drone.** A slowly modulated sub-100 Hz tone, sine + a touch of breath noise. Always on in the End, level proportional to distance from the central pillar (closer = louder).
- **Enderman whisper.** A very low faint pseudo-speech far off (the M7 ingressive pseudo-speech model, played at low extents and at the lowest of Foley's gain levels, so it's felt more than heard). Anchored to the central island.
- **Dragon bed.** A subsonic wingbeat bed at the central island, with the dragon's roar (M10) laid in at a slow Poisson rate. The bed fades with distance from the pillar.

**Exit:** on the central island the whisper is around you; flying through the outer islands the drone is there alone; near the dragon's perch the wingbeats rise; killing the dragon silences the wingbeat and roar beds but not the drone.

**Tests:** spawn on the central island, check the whisper + dragon bed; fly to the outer End, check only the drone; the wingbeat bed stops when the dragon dies; respawning the dragon restores it.

## M17 results: surf as a placed bed (2026-10-16)

- **The bed:** `FOLEY_BED_SURF = 9`, a streaming bed in `crates/foley-synth/src/surf.rs` (`Surf`/`SurfScene`) modelling the wave cycle: grains of the existing `outdoors::surf(height, seed)` scheduled by the bed itself at an interval mapped from `storm × exposure` — about 12 s in calm, about 2 s in a full storm — over a continuous band-passed shingle/foam wash (`≈ 0.15 × height/3 × shore`). At `height <= 0` it renders exact silence. Landed in ABI **23** (from 22).
- **Scene floats:** `{height, shore, storm, exposure}` — height in wave metres (the 0.2–3.0 tiers are `{0.3, 1.0, 2.2}`), the rest 0..1.
- **Driving it:** `SurfController` (10-tick cadence) samples distance-to-shore, salt vs fresh from `BiomeTags.IS_OCEAN`/`IS_DEEP_OCEAN`/`IS_BEACH`, storm from rain/thunder, and exposure from the wind bed's reported speed as `clamp(speed / 16f, 0, 1)` (16 m/s = `WindScene.STORM`). It drives one `BedSound<SurfScene>`, gated on the existing `water` toggle — no new config field. `findShore` moved from `Life` into `SurfController` and now skips unloaded chunks.
- **Life's one-shot is gone:** `Life` no longer fires surf; its `nextWave`/`shoreFoundAt`/`shore` fields and its `findShore` are removed. `Life.SURF_M` and `FOLEY_SHOT_SURF` remain for the addon/API path (`GeneratedSources` → `AddonSounds`); the `surf_0/1/2` sounds.json events were dropped in the coastline review once `GeneratedSounds` stopped rendering them, so they can no longer fall through to `otherwise`. sounds.json gains a `"surf"` bed event with the same `fabric-sound-api-v1:empty` streaming placeholder as `"biome"`.
- **Exit, met:** a stormy shore is continuous surf rather than discrete crashes; a lake laps quietly in light air; a calm sea keeps its slow surge; freshwater is gentler than salt at the same wind (the `heightFor` height tier, not the shore float); and with `Life`'s one-shots removed the bed does not double up.
- **Tests:** eight in `surf.rs`, including the foam-wash and wave-cadence pins; six `SurfControllerTest`; a `LifeTest` pinning the removal. All green: `cargo test --workspace --lib` (foley-synth 73, foley-dsp 29, foley-ffi 19 + 1 ignored, foley-math 0) and 78 Java tests (`gradlew test`). A client gametest checks the bed comes alive at a shore (with wind forced past the old 1.5 m/s firing gate), fires no `surf_*` one-shot over a 15 s window, and stops far inland; it runs CI-side only (Linux/Xvfb) and has not been executed locally.
- **Grain-spawn cost, measured:** `spawn_grain` synthesises a full ~7 s grain synchronously on the frames that spawn — worst `render` ≈11 ms over 800 renders (200 s of audio, release), first ≈7.9 ms, ~109/800 over 500 µs. Each pull is 0.25 s of audio, so the burst is ≈24× realtime; acceptable for now, and the pull size is the thing to watch.
- **Lake/ocean tier is read at the listener's biome** (`heightFor`), matching the old one-shot: a listener standing on a non-beach coastline beside an ocean gets the lake tier. Accepted.
- **Mirrors added:** `SurfController.exposureFor` (wind speed → 0..1, divisor 16 = `WindScene.STORM`) and the Java `surgeRate` mirror of the native surge cadence, both unit-pinned in `SurfControllerTest`.
- Released as **0.1.0-beta.57**.

## M18 plan: aeolian tones wired to placement

M1's wind bed has the Strouhal `0.2·U/d` model for fences, bars and leaves listed in its plan but never wired to objects in the world.

**Approach.** The wind scene gains an "aeolian structure" list: every iron bar, fence, lantern, bell and bamboo within 24 blocks of the ear, each with its characteristic diameter d, and the wind speed U at its height. Each one becomes a Strouhal whistle at `f ≈ 0.2·U/d`, panned to its position. The whistle is louder the higher U is; for U < 2 m/s it's silent.

Leaves rustling already exists in M1's turbulence model but uses a generic exposure factor. M18 ties it to leaf-bearing blocks (every `BlockTags.LEAVES` in the round) instead — more leaves round you, louder the rustle.

**Exit:** standing next to a fence in the wind sings; a bell rings in a storm; a bamboo grove howls; leaves in a tree rustle as the wind passes; spectrogram of the whistle shows the predicted Strouhal frequency.

**Tests:** place a row of iron bars in the wind, hear the Strouhal whine; place a bell at a windy height, hear it ring; stand under a bamboo grove, hear the howl; for a known U and d, f is within 10 % of the Strouhal prediction.

## M19 plan: biome-specific beds

Some biomes still sound like every other biome. M19 adds:

- **Desert / badlands / desert hills / desert lakes:** dry sand hiss underfoot — a granular bed, fired while the player walks on sand, level ∝ speed (the existing granular model from M5 footsteps, made continuous). A hollow wind: a higher-Q whistle of the M1 wind bed only in the open plain. Optional dust devils: small vortex rings rendered at a low Poisson rate (1 every 30 s), level ∝ U⁵, only when U > 8 m/s.
- **Mushroom fields:** a low ambient hum unique to the biome — a slowly modulated tone at ~120 Hz with a soft overtone at 1.6×, very quiet, fading to silent outside the biome.
- **Lush cave / dripstone cave:** the cave bed (M12) stays, but each gets a colour. Lush: a faint, soft high-frequency "dripping" hiss from the moss (band-passed noise at ~3 kHz, very low level, slow envelope). Dripstone: more drips, less rumble (drip rate × 2, rumble level × 0.5).
- **Deep dark:** a very low subsonic hum (~30 Hz) and the scuttling of sensors as one-shots every 30 s or so, played at a random sculk sensor within 24 blocks. Otherwise the bed stays silent — silence is the design.
- **Snowy / ice spikes / frozen ocean:** a "hush" bed — high-frequency absorption (above 4 kHz cut by a slow LPF) layered into the wind bed only in cold biomes, so snow absorbs the highs. The same LPF is applied to Foley's other beds in cold biomes.
- **Mangrove swamp:** a humid insect drone — band-passed noise at 4 kHz, level proportional to mangrove leaf count within 24 blocks, layered under the existing frogs and crickets.

**Exit:** each biome listed sounds distinct; the open-air biomes (plains, savanna, taiga) keep their M13 identity.

**Tests:** spawn in each, check the right bed plays; in a desert hear the sand hiss walking and the hollow wind; in a mushroom field hear the hum; in a deep dark hear the hum and the sensor scuttles; the snowy hush is audible above 4 kHz in a cold biome, absent in a warm one.

## M20 plan: village ambience

A village heard from 50 m should sound like a village. The pieces:

- **The village bell.** One of the most recognisable vanilla sounds; Foley doesn't currently replace or layer it. The bell is its own one-shot, played from the bell block; in a storm the bell rings more often (a real bell in a storm rings at random from sympathetic vibration).
- **The village murmur.** A low, far-off layer of voices and animal calls, panned to the village's centre (the bell block). Each layer is a low-pass filter (cutoff at 800 Hz) of the relevant voice. Anchored to the village's centre; audible within 50 m of the village.
- **Smith sounds.** An anvil ring (M4 modal) + a series of hammer-blow ticks at a slow Poisson rate, played from the smithy's location.
- **Scattered animals.** A chicken here, a cow lowing there, a wolf howl — placed at the village's structures (chickens at coops, cows by fences), not by mob spawn. The bird model already places by trees; this is "place by structure".

The village's centre (bell block) is the anchor: distance to the centre drives the bell's level and the murmur's panning. The murmur is one shared sound per village (not per mob), so a village doesn't sound like a chorus of 30 chickens.

**Exit:** a village heard from 50 m away is recognisably a village: bell + murmur + a chicken here, an anvil there; the bell rings more often in a storm; the smith's anvil is heard across the village.

**Tests:** spawn 50 m from a village, hear the bell + the murmur; at the smithy, hear the anvil ring; in a storm the bell rings more often; the murmur doesn't multiply with village size (it's panned, not summed).

## M21 plan: active structures

Sounds from objects that exist in the world that Foley doesn't touch:

- **Minecarts on rails.** The wheel-impact modal model (M4) is wired to minecarts. Each minecart has a per-tick wheel-impact one-shot, louder when powered (powered-rail tick at 2× the level). The "rush" when a minecart goes past at speed is a single bed per minecart, gain ∝ speed.
- **Doors.** Wooden doors open and close with stick–slip hinge creaks (M7's creak model). Iron doors and trapdoors get metallic creaks (M4's metal modes).
- **Chests / trapped chests / shulker boxes.** Opening with a small puff of air (the existing drips model with a different envelope — short attack, slow decay). Trapped chests get a faint redstone click.
- **Redstone.** A faint mechanical click when a repeater or comparator changes state, especially in clock circuits. A short stick–slip tick at every state change.
- **Boats.** A small placed bed per boat, water lapping at the hull + creaking wood (M4 wood modes). Gain falls off the further the boat is from shore.
- **Sculk sensors.** A triggered bed of low-Q tones played from the sensor and from nearby sculk blocks; the louder the trigger, the louder the ring.

**Exit:** a minecart system is heard before it's seen; an old wooden door creaks; a shulker clicks open; a redstone clock has a quiet rhythm; a sculk sensor "talks" to nearby sculk.

**Tests:** place a minecart on a rail, hear it roll past; open a chest, hear the puff; trigger a sculk sensor, hear it ring and the sculk around it; spectrogram of a redstone clock shows the ticks at the right rate; a wooden door and an iron door sound different.

## M22 plan: subtle physical layers

Density rather than new models:

- **Settling after footsteps and block placement.** A tiny delayed tick (10 to 30 ms after the footfall's thud) for gravel shifts, sand settles, snow brushes. Plays on the same logic as the existing `Brush` class, but for ground materials. Off when the footstep is on solid stone.
- **Mob brushes through plants.** Only the player brushes today (the `Brush` class). M22 extends the same scan to mobs: sheep wading through tall grass, a spider on cobwebs, a chicken in a flower field. Same plant classification, slower envelope (a mob moves slower through plants than a player running), quieter.
- **Peaceful-mob breathing.** Only zombies breathe when close (M7). M22 adds breathing for cows, sheep, pigs and villagers at very low gain, only in enclosed spaces (a barn, a pen, a village at night). The breath is a soft inhalation, ~10 a minute, gain 0.05.
- **Ash and snow drifting.** Almost inaudible but the right *texture* — high-frequency bandpassed noise at very low level during a blizzard or ash storm. Layered onto the wind bed's turbulence when the biome's weather is storming.

**Exit:** the world feels lived-in: footsteps settle, mobs interact with their world, peaceful nights are warm, blizzards and ash storms have texture.

**Tests:** walk through gravel, hear a delayed tick; a sheep walks through tall grass, hear a slower brush; stand outside a barn at night, hear a cow breathing very low; a blizzard has a high-frequency texture above the wind; the mob brush is quieter than the player's, never crowded.

## M23 results: note block instruments (2026-10-17)

- **The primitive:** `FOLEY_PRIMITIVE_NOTE` renders any of the sixteen classic instruments at any of a note block's twenty-five notes (0..24) on demand — `note(instrument, note, character, seed) -> buffer | null`. Landed in ABI **24** (from 23). The Java mirror is `NativeBridge.PRIMITIVE_NOTE`; `NoteBlocks.INSTRUMENTS` is the twin of the native table's order.
- **Five families, sixteen instruments:** every instrument is modelled, not resampled. **Pluck** (Karplus-Strong delay line, lowpass damping, per-instrument brightness) covers harp, bass, guitar and banjo. **Modal** (a bank of inharmonic modes — bar ratios 1, 2.76, 5.40, 8.93 — each with its own damping) covers bell, chime, xylophone, iron_xylophone and cow_bell — the bars short, the bells long and inharmonic, the cowbell clanky. **Membrane** (a bending drum head plus noise) covers basedrum, snare and hat, the drop and noise mix setting each. **Breath** (a tube with a jet and its formants) covers flute and didgeridoo. **Electronic** covers bit and pling, from a square-ish oscillator with a little analog drift. Every instrument carries its family's voicing in the table; `character` (0..1) then swings each call's timbre and ring length — duller and shorter low, brighter and longer high — across all five families. A note's frequency is the instrument's base (its note 12) times `2^((note − 12)/12)`, so the whole 0..24 range spans half to double the base.
- **The note:** `NoteBlocks.noteAt` reads the note block's `note` state at the strike's place. Where the block cannot be read (no level, an unloaded chunk, a strike played straight through `SoundManager.play` outside the world), `noteOf` falls back to the instance's pitch with vanilla's law, `12 + 12·log₂(pitch)`, clamped to 0..24. Pitch 1.0 reads as note 12, vanilla's default tuning.
- **The instrument** comes from the sound id — `block.note_block.<instrument>` — not from the block beneath, which vanilla resolved before the sound was built. An id Foley does not know is left to the game. In 26.3 there are instruments beyond pling (the trumpets, the mob-head imitations); Foley models the classic sixteen and those fall through to vanilla.
- **No sounds.json entry:** the strike plays `foley:generated/note_<instrument>_<note>_<variant>` through `AddonSounds.instance`, the path that resolves a generated buffer by name without a `sounds.json` event behind it (the same route the addon API uses). The event that names it, `foley:note`, has no `sounds.json` entry either. The **variant** (eight timbres per note) comes from the instance's identity hash: a strike is a fresh instance each time, and the sound's seed is not readable in these mappings, so successive hits differ by identity rather than by seed.
- **The toggle:** `noteBlocks` (index 19 of `FoleyConfig`, default on) sits in Options → Music & Sounds → Foley. Off, `Replacements` declines the strike and the game's own note plays. On, Foley plays its buffer and `SoundEngineMixin` skips the game's. A `NoteBlocks.plays()` counter records each strike Foley actually played, for the gametest — `FoleyClient.replaced()` on its own cannot tell a note Foley took from one it skipped.
- **Tests:** Rust — the instrument table's family mapping and the unknown-instrument boundary, the pitch law at note 12, and a measured-fundamental pin per family across the range (`note.rs`), plus the rendered-buffer path through `synthesize`. Java — `NoteBlocksTest` pins the buffer name, the unknown-instrument pass-through, a taken harp strike, and the pitch fall-back; the config round-trip and the Foley screen cover the toggle. All green: `cargo test --workspace --lib` (99 synth, 29 dsp, 22 ffi + 1 ignored) and 84 Java tests, with `compileGametestJava` compiling. A client gametest places a note block on gold, strikes the bell and asserts Foley played it (counter up, game's strike skipped), then switches `noteBlocks` off and asserts Foley neither plays nor replaces; it runs CI-side only (Linux/Xvfb) and has not been executed locally.

## M24 onward: vanilla takeover, family by family (2026-10-17)

**Policy:** when Foley synthesizes a family of sounds and the family's toggle is on, Foley **replaces** the vanilla events for that family — the game's own sounds are skipped — exactly as rain, thunder, ambience, drips, doors, chests, shulkers, minecarts and note blocks already do. Global bypass always lets vanilla through, and every family goes through the generic override registry (beta.60) so other mods can take it in turn. A family Foley only *layers* today is not finished until it replaces.

## M24 results: footsteps (2026-10-18)

- **Done:** `Replacements` now skips `block.<material>.step` while `footsteps` is on, through the standard family shape (bypass → vanilla; override verdict; then the toggle) — Foley's `Layers` step is the replacement, and the double-up is gone. The id rule and verdict live in a new package-private `Footsteps` class rather than in `Replacements`, because `Replacements`' statics load `SoundEvents`, which cannot class-init in plain unit tests (same reason `DoorHandling` uses literal ids).
- **Deliberately not skipped:** the same materials' `.fall` (landing) and `.hit` (dig tap) sounds. `Layers` gates those on the `strikes` setting, not `footsteps`, so skipping them here would silence landings whenever strikes is off; they join `.break`/`.place` in M26's `blocks` takeover. Entity steps stay vanilla (Decided #2).
- **Tests:** `FootstepsReplacementTest` (9 tests) pins a 16-material step table, sibling sounds not matched, entity/foreign ids not matched, toggle off → vanilla, override owns/declines, bypass neutral. Full suite 118 green.

## M25 results: note blocks join the override registry (2026-10-18)

- **Done:** `NoteBlocks.replace` now consults `DoorHandling.dispatch` after the bypass check and before the `noteBlocks` toggle — handled → skip, declined → vanilla, no match → the built-in strike and its toggle, identical to every other family. Unknown instruments still fall through to vanilla. Three tests (`anOverrideWinsOverTheBuiltInStrike`, `aDecliningOverrideLeavesTheStrikeToVanilla`, `anUnrelatedOverrideDoesNotOwnAStrike`) pin the wiring; `NoteBlocksTest` 9/9 green. The takeover table in `docs/INTEGRATING.md` gained its note-block row. The note family keeps its own `PLAYS` counter (an override-handled strike is the addon's, not Foley's).

## M26 plan: block break, place and digging

- Skip vanilla break/place/dig events when a new `blocks` toggle is on, synthesizing from Foley's contact/material primitives (`renderOneShot` contact taps, settling stone).
- Start narrow: stone, wood, gravel, sand, glass — the loudest and most frequent; other materials fall through to vanilla.
- **Tests:** per-material taken/declined, toggle, bypass, override; no double-sound.

## M27 plan: explosions

- Skip vanilla explosion events (creeper, TNT, end crystal, firework) when an `explosions` toggle is on; synthesize a physical bang (low boom + debris) with distance scaling.
- **Tests:** taken/declined per event, distance scaling sanity, toggle, bypass, override.

## M28 plan: redstone, boats, sculk as replacements

- Today these are additive layers (`Layers`/`ActiveStructures`/`Brush`) under still-playing vanilla events. When the family's toggle is on, skip the vanilla counterpart so Foley replaces rather than doubles: redstone (pistons, levers, buttons), boats (paddles, splash), sculk (shrieker, catalyst).
- **Exit:** the policy above holds for every family; audited against the takeover table in `docs/INTEGRATING.md`, which grows a row per family.

## Out of scope (decided)

- Mob one-shots (hurt, death, steps) stay vanilla — see Decided #2.
- Music discs, UI, eating, potions, elytra, villager chatter: no synthesis planned; they stay vanilla unless a later milestone claims them.

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
