//! Writes every mob's calls to WAV files, for listening outside the game:
//! `cargo run --release -p foley-synth --example voice_samples -- <dir>`.

use foley_synth::{to_pcm, voice, Call, Mob, SAMPLE_RATE};

fn main() -> std::io::Result<()> {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "samples".into());
    std::fs::create_dir_all(&dir)?;
    let calls = [Call::Ambient, Call::Hurt, Call::Death];
    let mut written = 0;
    for mob in Mob::ALL {
        let mut extra: Vec<Call> = match mob {
            Mob::Wolf => vec![Call::Growl, Call::Pant, Call::Whine],
            Mob::Cat => vec![Call::Purr, Call::Hiss, Call::Beg],
            Mob::Creeper => vec![Call::Primed],
            Mob::Zombie | Mob::Husk | Mob::ZombieVillager => {
                vec![Call::Breath, Call::BreathHeavy, Call::AmbientNear]
            }
            Mob::Drowned => vec![Call::AmbientWater, Call::BreathHeavy],
            Mob::Horse => vec![Call::Angry, Call::Snort],
            Mob::Donkey | Mob::Llama => vec![Call::Angry],
            Mob::Goat => vec![Call::Scream],
            Mob::Fox => vec![Call::Screech, Call::Aggressive, Call::Snort, Call::Sleep],
            Mob::Bee => vec![Call::Loop, Call::LoopAggressive],
            Mob::PolarBear => vec![Call::Warning, Call::AmbientBaby],
            Mob::Panda => vec![Call::Worried, Call::Aggressive, Call::Sneeze],
            Mob::Villager => vec![Call::Yes, Call::No, Call::Trade, Call::Celebrate],
            Mob::WanderingTrader => vec![Call::Yes, Call::No, Call::Trade],
            Mob::Enderman => vec![Call::Stare, Call::Scream],
            Mob::Witch | Mob::Pillager | Mob::Vindicator | Mob::Evoker => vec![Call::Celebrate],
            Mob::Slime | Mob::SlimeSmall => vec![Call::Squish, Call::Jump, Call::Attack],
            Mob::Phantom => vec![Call::Swoop, Call::Flap],
            Mob::Ravager => vec![Call::Roar, Call::Stunned],
            Mob::Vex => vec![Call::Charge],
            Mob::Guardian | Mob::ElderGuardian => vec![Call::AmbientLand, Call::Flop],
            Mob::Turtle => vec![Call::AmbientLand],
            _ => vec![],
        };
        extra.extend(calls);
        for call in extra {
            // Three renditions in a row, half a second apart.
            let mut pcm = Vec::new();
            for seed in 1..=3 {
                pcm.extend(
                    voice(mob, call, seed, foley_dsp::DEFAULT_SAMPLE_RATE)
                        .iter()
                        .map(|s| to_pcm(*s)),
                );
                pcm.extend(std::iter::repeat_n(0i16, SAMPLE_RATE as usize / 2));
            }
            let name = format!("{dir}/{mob:?}-{call:?}.wav").to_lowercase();
            std::fs::write(&name, wav(&pcm))?;
            written += 1;
        }
    }
    println!("wrote {written} files to {dir}");
    Ok(())
}

fn wav(pcm: &[i16]) -> Vec<u8> {
    let data = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.extend(b"RIFF");
    out.extend((36 + data).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(1u16.to_le_bytes());
    out.extend(SAMPLE_RATE.to_le_bytes());
    out.extend((SAMPLE_RATE * 2).to_le_bytes());
    out.extend(2u16.to_le_bytes());
    out.extend(16u16.to_le_bytes());
    out.extend(b"data");
    out.extend(data.to_le_bytes());
    for s in pcm {
        out.extend(s.to_le_bytes());
    }
    out
}
