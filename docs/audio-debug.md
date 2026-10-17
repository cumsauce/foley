# Audio debug logging (`audio-debug` feature)

Playtest instrumentation for the bed render path. Its only job is to show
*which* bed produces a suspicious blast (e.g. the "full-scale white noise"
report): it measures the raw synth peak before the level guard, the guard and
envelope state at the end of the block, the peak after the guard + underwater
low-pass, and the final PCM peak.

**Measurement only.** No audio behaviour changes: the logging is passive and
is compiled out entirely unless the feature is enabled.

## What it logs

Every bed prints one line to stderr every 12 000 frames — 0.25 s at the
crate's 48 kHz sample rate, about 94 of the 128-frame glide blocks the render
path works in, fine enough that a single loud block shows up as a spike
between two lines:

```
[foley] bed=surf env=0.83 guard=1.000 pre=0.912 post=0.364 pcm=11945
```

| field | meaning |
|-------|---------|
| `bed`  | the bed's kind (`BedKind` name, e.g. `surf`, `rain`, `wind`) |
| `env`  | `bed.envelope` at block end — the guard's envelope follower |
| `guard`| `bed.guard` at block end — the level guard's current gain |
| `pre`  | peak `abs()` of the raw synth sample **before** the guard/LPF |
| `post` | peak after the guard and the underwater low-pass |
| `pcm`  | peak `abs()` of the final `i16` samples from `to_pcm` |

A healthy block shows `post ≈ pre * guard` (after the LPF) and
`pcm ≤ 32767`. A block whose `pre` is at full scale while `guard` is still
ramping — or one whose `pcm` pins at 32767 — is the blast.

Because the numbers are peaks over that ~0.25 s window, a one-block
transient appears as a single spike in `pre`/`pcm` between otherwise quiet
lines.

## Building with it

The feature lives on `crates/foley-ffi` and is **not** default — the default
build contains zero instrumentation (every line is
`#[cfg(feature = "audio-debug")]`), so the realtime/no-alloc tests are
unaffected.

```sh
cargo build -p foley-ffi --features audio-debug
cargo test  -p foley-ffi                 # default features, must stay green
cargo clippy -p foley-ffi --all-targets --features audio-debug -- -D warnings
```

### Playtest jar

`mod/build.gradle` reads the Gradle property `foley.audioDebug`: when it is
`true`, the `cargoNatives` task (which stages the natives into the jar) sets
`FOLEY_AUDIO_DEBUG=1`, and `cargo xtask natives` turns that into
`--features audio-debug` for the `cargo build` of `foley-ffi`. Nothing is
edited and the property is absent from `gradle.properties`, so an ordinary
build stays default:

```sh
cd mod && ./gradlew build -Pfoley.audioDebug=true              # jar
cd mod && ./gradlew runClientGameTest -Pfoley.audioDebug=true  # playtest
```

Without Gradle (same thing by hand):

```sh
FOLEY_AUDIO_DEBUG=1 cargo xtask natives --target host --out target/dist/natives
cd mod && ./gradlew build -Pfoley.prebuiltNatives=../target/dist/natives
```

## Reading the output

Lines go to **stderr** (the game log's stderr), one per ~32 blocks per bed.
Filter with:

```sh
grep '^\[foley\] bed=' playtest.log
```

- Compare beds: the bed whose `pre` jumps to ~1.0 while its `pcm` saturates is
  the source of the blast.
- `env` climbing while `guard` stays near `BED_CEILING` means the guard has
  not caught up with a full-scale onset yet — that is the window a blast slips
  through.
- `guard` permanently below 1.0 with large `pre` means the guard is
  permanently attenuating a hot bed (quieter, not louder).
