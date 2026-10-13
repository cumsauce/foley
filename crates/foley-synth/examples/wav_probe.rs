//! Takes apart a recording of the game (OpenAL Soft's wave writer, 16-bit
//! stereo): splits it where it falls silent for over half a second, and for
//! each part prints its level, its energy per octave and how noise-like its
//! highs are (spectral flatness from 2 to 12 kHz: 1 is white noise, near 0 a
//! few tones). Also writes a spectrogram of the whole, 0–16 kHz. Parts can be
//! named instead, as name:start:end in seconds after the paths.
//! `cargo run --release -p foley-synth --example wav_probe -- in.wav out.png [name:start:end ...]`

const N: usize = 4096;
const OCTAVES: [f32; 8] = [125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let bytes = std::fs::read(&args[1])?;
    // Find the data chunk.
    let mut at = 12;
    let (mut rate, mut channels) = (48_000u32, 2u16);
    let data = loop {
        let id = &bytes[at..at + 4];
        let len = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        if id == b"fmt " {
            channels = u16::from_le_bytes(bytes[at + 10..at + 12].try_into().unwrap());
            rate = u32::from_le_bytes(bytes[at + 12..at + 16].try_into().unwrap());
        }
        if id == b"data" {
            let end = (at + 8 + len).min(bytes.len());
            break &bytes[at + 8..end];
        }
        at += 8 + len;
    };
    let ch = channels as usize;
    let mono: Vec<f32> = data
        .chunks_exact(2 * ch)
        .map(|f| {
            (0..ch)
                .map(|c| i16::from_le_bytes([f[2 * c], f[2 * c + 1]]) as f32 / 32768.0)
                .sum::<f32>()
                / ch as f32
        })
        .collect();
    let rate_f = rate as f32;
    println!("{:.1} s at {rate} Hz", mono.len() as f32 / rate_f);
    // Silences: 50 ms windows under -70 dBFS for over 0.6 s.
    let win = (0.05 * rate_f) as usize;
    let loud: Vec<bool> = mono
        .chunks(win)
        .map(|w| (w.iter().map(|x| x * x).sum::<f32>() / w.len() as f32).sqrt() > 3e-4)
        .collect();
    let mut parts = Vec::new();
    let (mut start, mut quiet) = (0usize, 0usize);
    for (i, l) in loud.iter().enumerate() {
        if *l {
            if quiet >= 12 {
                parts.push((start, i - quiet));
                start = i;
            }
            quiet = 0;
        } else {
            quiet += 1;
        }
    }
    parts.push((start, loud.len()));
    // Or the parts asked for, as name:start:end in seconds.
    let named: Vec<(String, usize, usize)> = args[3..]
        .iter()
        .map(|p| {
            let f: Vec<&str> = p.split(':').collect();
            let t = |x: &str| (x.parse::<f32>().unwrap() * rate_f / win as f32) as usize;
            (f[0].to_string(), t(f[1]), t(f[2]))
        })
        .collect();
    let parts: Vec<(String, usize, usize)> = if named.is_empty() {
        parts
            .iter()
            .enumerate()
            .map(|(k, (a, b))| (format!("part {k}"), *a, *b))
            .collect()
    } else {
        named
    };
    for (k, a, b) in &parts {
        let s = &mono[(a * win).min(mono.len())..(b * win).min(mono.len())];
        if s.len() < N * 4 {
            continue;
        }
        let rms = (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt();
        let peak = s.iter().fold(0f32, |m, x| m.max(x.abs()));
        // Welch: mean power spectrum over frames.
        let mut power = vec![0f32; N / 2];
        let frames = (s.len() - N) / (N / 2);
        for f in 0..frames {
            let m = magnitudes(&s[f * N / 2..f * N / 2 + N]);
            for (p, x) in power.iter_mut().zip(m) {
                *p += x * x;
            }
        }
        let hz = |bin: usize| bin as f32 * rate_f / N as f32;
        let bands: Vec<String> = OCTAVES
            .iter()
            .map(|c| {
                let e: f32 = (0..N / 2)
                    .filter(|&b| hz(b) >= c / 1.414 && hz(b) < c * 1.414)
                    .map(|b| power[b] / frames as f32)
                    .sum();
                format!("{:>5.0}", 10.0 * e.max(1e-20).log10())
            })
            .collect();
        let hi: Vec<f32> = (0..N / 2)
            .filter(|&b| hz(b) >= 2000.0 && hz(b) <= 12000.0)
            .map(|b| power[b].max(1e-20))
            .collect();
        let geo = (hi.iter().map(|p| p.ln()).sum::<f32>() / hi.len() as f32).exp();
        let arith = hi.iter().sum::<f32>() / hi.len() as f32;
        println!(
            "{k:>28}: {:6.1}-{:6.1} s  rms {:6.1} dBFS  peak {:6.1}  flatness {:.2}  octaves 125..16k dB: {}",
            (a * win) as f32 / rate_f,
            (b * win) as f32 / rate_f,
            20.0 * rms.max(1e-9).log10(),
            20.0 * peak.max(1e-9).log10(),
            geo / arith,
            bands.join(" ")
        );
    }
    // Spectrogram of the whole, 0–16 kHz.
    let width = 1200usize;
    let bins = N / 2 * 16000 / (rate as usize / 2);
    let height = 300usize;
    let hop = ((mono.len() - N) / width).max(1);
    let mut image = vec![0u8; width * height];
    for x in 0..width.min((mono.len() - N) / hop) {
        let m = magnitudes(&mono[x * hop..x * hop + N]);
        for y in 0..height {
            let b = y * bins / height;
            let db = 20.0 * m[b].max(1e-7).log10();
            image[(height - 1 - y) * width + x] =
                (((db + 100.0) / 80.0).clamp(0.0, 1.0) * 255.0) as u8;
        }
    }
    std::fs::write(&args[2], png(&image, width, height))
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
