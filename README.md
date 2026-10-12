# Foley

Procedurally generated, physically based sound effects and ambience for Minecraft 26.3. It's a client-only Fabric mod: rain that sounds different on leaves, stone, water and a roof; wind that rises with height and exposure; streams, waterfalls, cave drips, fire and lava, synthesized from physical models rather than played from recordings.

It runs alongside [Earshot](https://github.com/cumsauce/earshot), which shapes every sound Foley makes with its occlusion, reverb and HRTF. It also runs alongside [Yak VC](https://github.com/desocketed/yakvc), which it doesn't touch.

Status: **M2, water, fire and lava.**

- **Rain** replaces the game's rain sounds. Drops fall at the rate the weather gives (about 10 mm/h in rain, 50 in a thunderstorm), sized by the Marshall–Palmer distribution, onto what's actually round you: water rings with bubbles, stone ticks, leaves patter, wood knocks, glass and metal ping, grass and sand thud softly. Under a roof you hear the drumming on the roof instead.
- **Water, fire and lava** replace the game's ambience within 12 blocks, each played from where it is, so it pans (and Earshot occludes it). Streams babble with bubbles of 1 to 4 mm; waterfalls roar where they land, with bubbles of every size; fire roars and puffs, and crackles in a heavy-tailed mix of ticks and pops; lava rumbles, and its bubbles burst and ring as the cavities they leave. Still water is silent.
- **Dripstone drips** are generated: into water (the bubble rings), on stone (a tick), or lava (a thud and a sizzle).
- **Wind** layers on top of the game's sounds. It rises with height above the ground round you (the log wind profile), drops in a forest or a valley, gusts, rustles the leaves round you, rises when you fly, and comes in low and muffled indoors. None in the Nether or the End.

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
