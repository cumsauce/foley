# Foley

Procedurally generated, physically based sound effects and ambience for Minecraft 26.3. It's a client-only Fabric mod: rain that sounds different on leaves, stone, water and a roof; wind that rises with height and exposure; streams, waterfalls, cave drips, fire and lava, synthesized from physical models rather than played from recordings.

It runs alongside [Earshot](https://github.com/cumsauce/earshot), which shapes every sound Foley makes with its occlusion, reverb and HRTF. It also runs alongside [Yak VC](https://github.com/desocketed/yakvc), which it doesn't touch.

Status: **0.1.0-beta.3 (M7).** Mob voices are being remade, most-heard mobs first (docs/PLAN.md, M7–M11).

- **Rain** replaces the game's rain sounds. Drops fall at the rate the weather gives (about 10 mm/h in rain, 50 in a thunderstorm), sized by the Marshall–Palmer distribution, onto what's actually round you: water rings with bubbles, stone ticks, leaves patter, wood knocks, glass and metal ping, grass and sand thud softly. Under a roof you hear the drumming on the roof instead.
- **Water, fire and lava** replace the game's ambience within 12 blocks, each played from where it is, so it pans (and Earshot occludes it). Streams babble with bubbles of 1 to 4 mm; waterfalls roar where they land, with bubbles of every size; fire roars and puffs, and crackles in a heavy-tailed mix of ticks and pops; lava rumbles, and its bubbles burst and ring as the cavities they leave. Still water is silent.
- **Thunder** replaces the game's. Each flash's channel is drawn kilometres up into the cloud, and each piece of it sends out its own shock wave. Parts seen side on clap, parts seen end on rumble, and far parts lose their highs to the air. So a near strike cracks and then rolls on for seconds, and a far one arrives late and dull. It arrives as late as sound takes, with or without Earshot. Indoors, only the low rumble comes through.
- **Strikes and breaks** layer under the game's block sounds. Mining taps, placing, landing, breaking and footsteps each ring the block's own modes, from its material's stiffness, density and damping. Metal clangs and rings on, stone clicks, wood knocks, earth and wool thud, gravel crunches, and snow squeaks underfoot. Glass shatters into shards that fall and tinkle. The game's sounds stay.
- **Ambient life** layers on top. Each call plays from where the animal is.
  - Birds sing from the trees by day, most at dawn, not in the rain. Each biome has its own species.
  - Crickets chirp from the grass on warm nights, faster the warmer it is (Dolbear's law).
  - Frogs call from still water at night or in the rain; bigger ones in swamps.
- **Dripstone drips** are generated: into water (the bubble rings), on stone (a tick), or lava (a thud and a sizzle).
- **Wind** layers on top of the game's sounds. It rises with height above the ground round you (the log wind profile), drops in a forest or a valley, gusts, rustles the leaves round you, rises when you fly, and comes in low and muffled indoors. None in the Nether or the End.

- **Mob voices** (experimental, off until you choose them, group by group). Each voice comes from a physical model: a glottal source with the rough, chaotic features real distressed voices have, through a modelled vocal tract sized by the animal. Mobs with no voice sound like what they're made of.
  - Cows, pigs, sheep, chickens, villagers, wolves (growls, pants, whines) and cats (purrs, hisses).
  - Zombies, husks, drowned and zombie villagers: rough, breathy groans. They breathe heavier, faster and louder as they close in on you.
  - Skeletons: rattling bones and chattering teeth. Spiders: stridulation and hissing. Creepers: a swelling fuse hiss. Bats: squeaks.
  - Babies are higher, and bigger variants lower. `cargo run --release -p foley-synth --example voice_samples` writes every call to WAV files.

**Settings:** Options → Music & Sounds → Foley… (or Mod Menu). Foley's volume; each sound it replaces can be given back to the game, and each it adds switched off. Bind "Bypass Foley" in Controls to switch it all off and on while you listen.

Commands: `/foley rain <mm/h>|auto`, `/foley wind <m/s>|auto`, `/foley scene` (what Foley hears round you), `/foley drip`. [docs/PLAN.md](docs/PLAN.md) has the design and results.

## Development

You need JDK 25 and rustup (which reads `rust-toolchain.toml`). The layout follows Earshot's:

```sh
cargo test --workspace                                  # Rust crates
cargo xtask header                                      # regenerate crates/foley-ffi/include/foley.h after changing the C ABI
cargo run --release -p foley-synth --example bed_bench  # what synthesis costs
cd mod && ./gradlew runClientGameTest                   # the real game: rain, wind and drips
```

In a VM building from a VirtioFS shared folder, set `CARGO_TARGET_DIR` to a local disk and pass `-Pfoley.runDir=<local dir>` to game runs. Foley needs `--enable-native-access=ALL-UNNAMED` in the launcher's JVM arguments, like Earshot and Yak VC.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.
