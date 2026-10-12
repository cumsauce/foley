# Foley

Procedurally generated, physically based sound effects and ambience for Minecraft 26.3. It's a client-only Fabric mod: rain that sounds different on leaves, stone, water and a roof; wind that rises with height and exposure; streams, waterfalls, cave drips, fire and lava, synthesized from physical models rather than played from recordings.

It runs alongside [Earshot](https://github.com/cumsauce/earshot), which shapes every sound Foley makes with its occlusion, reverb and HRTF. It also runs alongside [Yak VC](https://github.com/desocketed/yakvc), which it doesn't touch.

Status: **M0, groundwork.** A synthesized rain-on-water stream and generated drips play in game, as a proof of the two paths (`/foley rain <drops a second>`, `/foley drip`). [docs/PLAN.md](docs/PLAN.md) has the design and results.

## Development

You need JDK 25 and rustup (which reads `rust-toolchain.toml`). The layout follows Earshot's:

```sh
cargo test --workspace                                  # Rust crates
cargo xtask header                                      # regenerate crates/foley-ffi/include/foley.h after changing the C ABI
cargo run --release -p foley-synth --example bed_bench  # what synthesis costs
cd mod && ./gradlew runClientGameTest                   # the real game, checking both playback paths
```

In a VM building from a VirtioFS shared folder, set `CARGO_TARGET_DIR` to a local disk and pass `-Pfoley.runDir=<local dir>` to game runs. Foley needs `--enable-native-access=ALL-UNNAMED` in the launcher's JVM arguments, like Earshot and Yak VC.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.
