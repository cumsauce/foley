//! Spectrograms of a few calls, as one PNG, for looking at what a voice is
//! made of: `cargo run --release -p foley-synth --example spectrogram -- out.png`.
//! Rows: zombie groan, zombie close by, cow, sheep, cat, wolf growl; or the
//! M8 or M9 sets (a second argument `m8` or `m9`), or the outdoors (`outdoors`:
//! surf, woodpecker, owl, cicadas, a wolf howling), or `birds` (crow, dove,
//! robin, cardinal, chickadee), or footsteps (`steps`:
//! stone, wood, grass, gravel, snow), or `drips` (into water, on stone, lava,
//! a small frog, a big one). 0–8 kHz, log magnitude;
//! each row spans its whole sound.

use foley_synth::{Call, Mob, cicadas, howl, owl, surf, voice, woodpecker};

const N: usize = 1024;
const HOP: usize = 256;
const BINS: usize = 171; // 0–8 kHz at 48 kHz
const WIDTH: usize = 400;

fn main() -> std::io::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "spectrogram.png".into());
    let set = std::env::args().nth(2).unwrap_or_default();
    let calls: Vec<(Mob, Call)> = if set == "m8" {
        vec![
            (Mob::Horse, Call::Ambient),
            (Mob::Donkey, Call::Ambient),
            (Mob::Goat, Call::Scream),
            (Mob::Llama, Call::Ambient),
            (Mob::Camel, Call::Ambient),
            (Mob::Bee, Call::Loop),
        ]
    } else if set == "m9" {
        vec![
            (Mob::Enderman, Call::Ambient),
            (Mob::Witch, Call::Celebrate),
            (Mob::Pillager, Call::Ambient),
            (Mob::Ravager, Call::Roar),
            (Mob::Guardian, Call::Ambient),
            (Mob::Slime, Call::Squish),
        ]
    } else {
        vec![
            (Mob::Zombie, Call::Ambient),
            (Mob::Zombie, Call::AmbientNear),
            (Mob::Cow, Call::Ambient),
            (Mob::Sheep, Call::Ambient),
            (Mob::Cat, Call::Ambient),
            (Mob::Wolf, Call::Growl),
        ]
    };
    let rows: Vec<Vec<f32>> = if set == "drips" {
        use foley_synth::{drip_into_water, drip_on_stone, frog, lava_drip};
        vec![
            drip_into_water(0.0015, 2),
            drip_on_stone(0.0025, 3),
            lava_drip(4),
            frog(4.0, 6),
            frog(12.0, 7),
        ]
    } else if set == "steps" {
        use foley_synth::{Contact, Material, contact};
        [
            Material::Stone,
            Material::Wood,
            Material::Plant,
            Material::Granular,
            Material::Snow,
        ]
        .iter()
        .map(|m| contact(Contact::Step, *m, 1.0, 3))
        .collect()
    } else if set == "birds" {
        foley_synth::Bird::ALL
            .iter()
            .map(|bird| foley_synth::bird_call(*bird, 1))
            .collect()
    } else if set == "outdoors" {
        vec![
            surf(1.5, 1),
            woodpecker(2),
            owl(3),
            cicadas(30.0, 4),
            howl(5),
        ]
    } else {
        calls
            .iter()
            .map(|(mob, call)| voice(*mob, *call, 1))
            .collect()
    };
    let height = rows.len() * (BINS + 4);
    let mut image = vec![0u8; WIDTH * height];
    for (r, sound) in rows.iter().enumerate() {
        let hop = ((sound.len().saturating_sub(N)) / WIDTH).max(HOP);
        let frames = ((sound.len().saturating_sub(N)) / hop).min(WIDTH);
        for f in 0..frames {
            let spectrum = magnitudes(&sound[f * hop..f * hop + N]);
            for (b, magnitude) in spectrum.iter().take(BINS).enumerate() {
                let db = 20.0 * magnitude.max(1e-7).log10();
                let v = ((db + 90.0) / 90.0).clamp(0.0, 1.0);
                let y = r * (BINS + 4) + (BINS - 1 - b);
                image[y * WIDTH + f] = (v * 255.0) as u8;
            }
        }
    }
    std::fs::write(path, png(&image, WIDTH, height))
}

fn magnitudes(frame: &[f32]) -> Vec<f32> {
    let mut re: Vec<f32> = frame
        .iter()
        .enumerate()
        .map(|(i, x)| x * (0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / N as f32).cos()))
        .collect();
    let mut im = vec![0.0; N];
    // In-place radix-2 FFT.
    let mut j = 0;
    for i in 1..N {
        let mut bit = N >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= N {
        let angle = -std::f32::consts::TAU / len as f32;
        for start in (0..N).step_by(len) {
            for k in 0..len / 2 {
                let (s, c) = (angle * k as f32).sin_cos();
                let (a, b) = (start + k, start + k + len / 2);
                let (tr, ti) = (re[b] * c - im[b] * s, re[b] * s + im[b] * c);
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        len <<= 1;
    }
    (0..N / 2)
        .map(|k| (re[k] * re[k] + im[k] * im[k]).sqrt() / N as f32)
        .collect()
}

/// A greyscale PNG, stored (uncompressed) deflate.
fn png(pixels: &[u8], width: usize, height: usize) -> Vec<u8> {
    fn crc(data: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for &b in data {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
        }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
        out.extend((data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend(data);
        out.extend(&body);
        out.extend(crc(&body).to_be_bytes());
    }
    let mut raw = Vec::new();
    for row in pixels.chunks(width) {
        raw.push(0);
        raw.extend(row);
    }
    let mut z = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65_535).enumerate() {
        let last = (i + 1) * 65_535 >= raw.len();
        z.push(u8::from(last));
        z.extend((block.len() as u16).to_le_bytes());
        z.extend((!(block.len() as u16)).to_le_bytes());
        z.extend(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + u32::from(x)) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend(((b << 16) | a).to_be_bytes());
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut header = Vec::new();
    header.extend((width as u32).to_be_bytes());
    header.extend((height as u32).to_be_bytes());
    header.extend([8, 0, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}
