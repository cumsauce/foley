use std::f32::consts::{PI, TAU};

use crate::{BandPass, LowPass, Rng, SAMPLE_RATE};

const SPEED_OF_SOUND: f32 = 343.0;
const GRAVITY: f32 = 9.81;

/// What a block is made of, as far as striking and breaking it goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Material {
    Stone,
    Wood,
    Metal,
    Glass,
    Ice,
    /// Dirt, grass, clay, mud.
    Earth,
    /// Sand and gravel.
    Granular,
    /// Leaves, plants, crops.
    Plant,
    /// Wool, slime, carpets.
    Soft,
    /// Snow: soft, but it crunches and squeaks as the bonds between its
    /// grains break.
    Snow,
}

impl Material {
    pub const ALL: [Material; 10] = [
        Material::Stone,
        Material::Wood,
        Material::Metal,
        Material::Glass,
        Material::Ice,
        Material::Earth,
        Material::Granular,
        Material::Plant,
        Material::Soft,
        Material::Snow,
    ];

    pub fn from_index(index: usize) -> Option<Material> {
        Material::ALL.get(index).copied()
    }

    fn props(self) -> Props {
        // Young's modulus (Pa), density (kg/m³), Poisson's ratio, loss
        // factor η as built in (more than the bare material's: blocks are
        // held by their neighbours), the vibrating plate (thickness, sides,
        // m), and how it breaks.
        let p = |young, density, poisson, loss, thickness, fragments, restitution, shard| Props {
            young,
            density,
            poisson,
            loss,
            thickness,
            fragments,
            restitution,
            shard,
        };
        match self {
            // Granite-like, a slab 25 cm thick: rings faintly above 1 kHz.
            Material::Stone => p(50e9, 2700.0, 0.25, 0.04, 0.25, 16, 0.35, 0.0),
            // A floor of 2.5 cm boards: a knock round 100 to 500 Hz.
            Material::Wood => p(10e9, 500.0, 0.3, 0.025, 0.025, 6, 0.4, 0.01),
            // Steel, 5 cm: a clang from 240 Hz ringing for seconds.
            Material::Metal => p(200e9, 7850.0, 0.3, 0.004, 0.05, 0, 0.6, 0.0),
            // A 1 cm pane: dense modes, bright when struck hard; shards 4 mm.
            Material::Glass => p(70e9, 2500.0, 0.22, 0.004, 0.01, 40, 0.5, 0.004),
            Material::Ice => p(9e9, 917.0, 0.33, 0.006, 0.25, 30, 0.45, 0.0),
            Material::Earth => p(50e6, 1600.0, 0.35, 0.3, 0.3, 0, 0.1, 0.0),
            Material::Granular => p(100e6, 1600.0, 0.3, 0.5, 0.3, 0, 0.1, 0.0),
            Material::Plant => p(0.5e9, 700.0, 0.3, 0.08, 0.002, 0, 0.2, 0.0),
            Material::Soft => p(1e6, 300.0, 0.3, 0.6, 0.3, 0, 0.1, 0.0),
            // Settled snow: a few MPa, a third the density of water.
            Material::Snow => p(5e6, 300.0, 0.3, 0.5, 0.3, 0, 0.1, 0.0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Props {
    young: f32,
    density: f32,
    poisson: f32,
    loss: f32,
    thickness: f32,
    /// Fragments it shatters into (0: it doesn't shatter).
    fragments: usize,
    /// Coefficient of restitution of a fragment bouncing.
    restitution: f32,
    /// Shards' thickness, m; 0 for chunks (as thick as half their size).
    shard: f32,
}

impl Props {
    /// The plate's speed of longitudinal waves, √(E / ρ(1 − ν²)).
    fn wave_speed(&self) -> f32 {
        (self.young / (self.density * (1.0 - self.poisson * self.poisson))).sqrt()
    }

    /// A simply supported plate's mode (m, n) on a × b:
    /// f = (π/2)·h·√(E / 12ρ(1 − ν²))·((m/a)² + (n/b)²).
    fn plate_mode(&self, m: f32, n: f32, a: f32, b: f32) -> f32 {
        PI / 2.0 * self.thickness * self.wave_speed() / 12f32.sqrt()
            * ((m / a).powi(2) + (n / b).powi(2))
    }

    /// The coincidence frequency, below which a plate radiates poorly:
    /// f_c = c² / (1.8·h·c_L).
    fn coincidence(&self) -> f32 {
        SPEED_OF_SOUND * SPEED_OF_SOUND / (1.8 * self.thickness * self.wave_speed())
    }
}

/// What strikes the block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Contact {
    /// A tool tapping it as it's mined: a steel head, 0.5 kg, 1 cm round.
    Tap,
    /// A block set down on it: 2 kg of the same stuff, 5 cm round.
    Place,
    /// A body landing on it: 70 kg on shoe soles (10 MPa), 10 cm round.
    Land,
    /// A footstep: a heel strike, about 8 kg of foot and shank on a 5 cm
    /// heel of shoe sole.
    Step,
}

impl Contact {
    pub const ALL: [Contact; 4] = [Contact::Tap, Contact::Place, Contact::Land, Contact::Step];

    pub fn from_index(index: usize) -> Option<Contact> {
        Contact::ALL.get(index).copied()
    }
}

/// Hertz's contact time of a sphere of mass `mass` and radius `radius`
/// meeting a surface at `speed`, with effective modulus `stiffness`:
/// τ = 2.87·(m² / (R·E*²·v))^⅕. The force is a half-sine that long, so
/// a hard, light, fast strike is short and bright, and a soft or heavy one
/// long and dull.
pub fn hertz_time(mass: f32, radius: f32, stiffness: f32, speed: f32) -> f32 {
    2.87 * (mass * mass / (radius * stiffness * stiffness * speed.max(0.01))).powf(0.2)
}

/// The effective modulus of two bodies in contact: 1/E* = Σ (1 − ν²)/E.
fn effective_modulus(a: (f32, f32), b: (f32, f32)) -> f32 {
    1.0 / ((1.0 - a.1 * a.1) / a.0 + (1.0 - b.1 * b.1) / b.0)
}

/// The spectrum of a half-sine force pulse `tau` long, against its DC:
/// |cos(πfτ) / (1 − (2fτ)²)|. Little gets through above 1/τ.
fn half_sine_spectrum(frequency: f32, tau: f32) -> f32 {
    let x = 2.0 * frequency * tau;
    let denominator = 1.0 - x * x;
    if denominator.abs() < 1e-3 {
        PI / 4.0
    } else {
        ((PI * frequency * tau).cos() / denominator).abs()
    }
}

/// A bank of damped modes, each a phasor (re, im) turned by (cos, sin) of
/// its step a sample and shrunk by its decay.
#[derive(Debug, Default)]
struct Modes(Vec<[f32; 4]>);

impl Modes {
    fn add(&mut self, frequency: f32, decay: f32, amplitude: f32) {
        let rate = SAMPLE_RATE as f32;
        if frequency >= 0.45 * rate || amplitude == 0.0 {
            return;
        }
        let shrink = (-decay / rate).exp();
        let step = TAU * frequency / rate;
        self.0
            .push([amplitude, 0.0, shrink * step.cos(), shrink * step.sin()]);
    }

    /// Adds the modes into `out` from `start`, until they die away.
    fn render_into(&mut self, out: &mut Vec<f32>, start: usize) {
        let mut i = start;
        loop {
            let mut sample = 0.0;
            let mut alive = false;
            for [re, im, c, s] in &mut self.0 {
                let (r, j) = (*re * *c - *im * *s, *re * *s + *im * *c);
                *re = r;
                *im = j;
                sample += j;
                alive |= r.abs() + j.abs() > 1e-5;
            }
            if i >= out.len() {
                out.resize(i + 1, 0.0);
            }
            out[i] += sample;
            i += 1;
            if !alive || i - start > 4 * SAMPLE_RATE as usize {
                break;
            }
        }
    }
}

/// The struck block's modes for a strike at a random point, weighted by how
/// the strike excites each (sin(mπx₀/a)·sin(nπy₀/b) for a simply supported
/// plate), the contact's force spectrum, and how well each radiates
/// (√(f / (f + f_c))). The 48 strongest under 16 kHz.
fn struck_modes(rng: &mut Rng, props: &Props, tau: f32, amplitude: f32) -> Modes {
    let (a, b) = (1.0, 1.0);
    let (x0, y0) = (0.1 + 0.8 * rng.unit(), 0.1 + 0.8 * rng.unit());
    let coincidence = props.coincidence();
    let mut modes = Vec::new();
    for m in 1..=40 {
        for n in 1..=40 {
            let f = props.plate_mode(m as f32, n as f32, a, b);
            if f > 16_000.0 {
                break;
            }
            if f < 20.0 {
                continue;
            }
            let shape = (m as f32 * PI * x0 / a).sin() * (n as f32 * PI * y0 / b).sin();
            // An impulse gives every mode the same velocity (over its
            // modal mass); what radiates is its acceleration, ∝ f.
            let weight =
                shape.abs() * f * half_sine_spectrum(f, tau) * (f / (f + coincidence)).sqrt();
            // A little mistuning, as real blocks aren't ideal plates.
            modes.push((f * (1.0 + 0.02 * rng.signed()), weight));
        }
    }
    modes.sort_by(|x, y| y.1.total_cmp(&x.1));
    modes.truncate(48);
    let strongest = modes.first().map_or(1.0, |m| m.1).max(1e-12);
    let mut bank = Modes::default();
    for (f, weight) in modes {
        bank.add(f, PI * f * props.loss, amplitude * weight / strongest);
    }
    bank
}

/// A fragment's lowest mode: 0.453·c_L·t / L² for one `size` m across.
fn fragment_frequency(props: &Props, size: f32) -> f32 {
    let thickness = if props.shard > 0.0 {
        props.shard.min(size)
    } else {
        size / 2.0
    };
    0.453 * props.wave_speed() * thickness / (size * size)
}

/// A small fragment's modes: a shard or chunk `size` m across rings at
/// f = 0.453·c_L·t / L² (a free plate's), with overtones at 1.6, 2.3, 3.1
/// times that.
fn fragment_modes(rng: &mut Rng, props: &Props, size: f32, amplitude: f32) -> Modes {
    let f1 = fragment_frequency(props, size);
    let mut bank = Modes::default();
    for (ratio, gain) in [(1.0, 1.0), (1.6, 0.7), (2.3, 0.5), (3.1, 0.35)] {
        let f = f1 * ratio * (1.0 + 0.05 * rng.signed());
        // Free, a shard loses less than the held block (half its η), but
        // lying on the ground it's damped at least 25 a second (gone in
        // under 0.3 s).
        let decay = (PI * f * props.loss * 0.5).max(25.0);
        bank.add(f, decay, amplitude * gain * (0.5 + 0.5 * rng.unit()));
    }
    bank
}

/// Grains: `count` tiny clicks over `seconds`, as sand or gravel shifts or
/// soil crumbles; `bright` from 0 (dull) to 1.
fn grains(
    rng: &mut Rng,
    out: &mut Vec<f32>,
    count: usize,
    seconds: f32,
    amplitude: f32,
    bright: f32,
) {
    let rate = SAMPLE_RATE as f32;
    for _ in 0..count {
        // Most of them early, as the pile settles.
        let at = (seconds * rng.unit().powi(2) * rate) as usize;
        let mut bank = Modes::default();
        let f = 800.0 + 6000.0 * bright * rng.unit();
        bank.add(f, 2000.0, amplitude * (0.3 + rng.unit()));
        bank.render_into(out, at);
    }
}

/// A burst of filtered noise from `start`: `seconds` long, decaying, through
/// a band round `centre`.
fn noise_burst(
    rng: &mut Rng,
    out: &mut Vec<f32>,
    start: usize,
    seconds: f32,
    centre: f32,
    amplitude: f32,
) {
    let rate = SAMPLE_RATE as f32;
    let length = (seconds * rate) as usize;
    let mut band = BandPass::new(centre, 1.2);
    if out.len() < start + length {
        out.resize(start + length, 0.0);
    }
    let decay = (-5.0 / length.max(1) as f32).exp();
    let mut envelope = amplitude;
    for sample in &mut out[start..start + length] {
        *sample += envelope * band.process(rng.signed());
        envelope *= decay;
    }
}

fn normalize(out: &mut [f32], peak: f32) {
    crate::dsp::normalize_in_place(out, peak);
    // A short fade so nothing ends on a click.
    let fade = out.len().min(240);
    let len = out.len();
    for (i, sample) in out[len - fade..].iter_mut().enumerate() {
        *sample *= 1.0 - i as f32 / fade as f32;
    }
}

/// How loud a material sounds struck, against stone: by its mechanical
/// impedance ρc, as soft stuff takes the blow without ringing.
fn loudness(props: &Props) -> f32 {
    let stone = Material::Stone.props();
    ((props.density * props.wave_speed()) / (stone.density * stone.wave_speed()))
        .powf(0.25)
        .clamp(0.2, 1.2)
}

/// A strike on a block of `material` (M4): the struck block's modes,
/// excited through the Hertz contact of what strikes it at `speed` m/s, and
/// the click of the contact itself. Grainy materials crunch too.
pub fn contact(contact: Contact, material: Material, speed: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let props = material.props();
    let peak = match contact {
        Contact::Tap => 0.35,
        Contact::Place => 0.5,
        Contact::Land => 0.7,
        Contact::Step => 0.4,
    };
    let mut out = strike(&mut rng, contact, material, &props, speed);
    match contact {
        Contact::Step => step_through(&mut rng, &mut out, material, &props, speed),
        Contact::Place => settle_in(&mut rng, &mut out, material, &props, speed),
        _ => {}
    }
    normalize(&mut out, peak * loudness(&props));
    out
}

/// The rest of a footstep after the heel: the sole scuffing as the foot rolls
/// forward, then the ball of the foot coming down 60 to 120 ms later, softer;
/// grass swishes round the shoe, and a wooden floor now and then creaks
/// under the weight.
fn step_through(rng: &mut Rng, out: &mut Vec<f32>, material: Material, props: &Props, speed: f32) {
    let rate = SAMPLE_RATE as f32;
    let roll = 0.06 + 0.06 * rng.unit();
    // The scuff: the sole sliding a little on the surface, grittier on stone
    // and gravel, nothing on soft things.
    let (grit, centre) = match material {
        Material::Stone | Material::Granular => (0.12, 3500.0),
        Material::Wood | Material::Metal | Material::Glass | Material::Ice => (0.06, 2500.0),
        Material::Earth => (0.05, 1500.0),
        Material::Plant => (0.0, 0.0),
        Material::Soft | Material::Snow => (0.02, 1200.0),
    };
    if grit > 0.0 {
        let at = (0.01 * rate) as usize;
        let centre = centre * (0.8 + 0.4 * rng.unit());
        noise_burst(rng, out, at, roll, centre, grit);
    }
    // The ball of the foot.
    let mut toe = strike(rng, Contact::Step, material, props, 0.6 * speed);
    let gain = 0.45 + 0.25 * rng.unit();
    toe.iter_mut().for_each(|x| *x *= gain);
    let at = (roll * rate) as usize;
    if out.len() < at + toe.len() {
        out.resize(at + toe.len(), 0.0);
    }
    for (o, t) in out[at..].iter_mut().zip(&toe) {
        *o += t;
    }
    match material {
        // Blades brushing the shoe as it goes in and comes out: a few soft
        // rustles over the step.
        Material::Plant => {
            for _ in 0..3 + (rng.unit() * 4.0) as usize {
                let at = ((0.02 + 0.2 * rng.unit()) * rate) as usize;
                let seconds = 0.03 + 0.05 * rng.unit();
                let centre = 3000.0 + 3000.0 * rng.unit();
                noise_burst(rng, out, at, seconds, centre, 0.15);
            }
        }
        // A board creaking: its fibres sticking and slipping against the
        // nails as the weight comes on.
        Material::Wood if rng.unit() < 0.15 => {
            let seconds = 0.12 + 0.15 * rng.unit();
            let n = (seconds * rate) as usize;
            let mut body = BandPass::new(700.0 + 900.0 * rng.unit(), 5.0);
            let mut hz = 90.0 + 80.0 * rng.unit();
            let mut phase = 0.0f32;
            let start = (0.03 * rate) as usize;
            if out.len() < start + n {
                out.resize(start + n, 0.0);
            }
            for i in 0..n {
                let t = i as f32 / n as f32;
                hz *= 1.0 + 0.003 * rng.signed();
                phase += hz / rate;
                let slip = if phase >= 1.0 {
                    phase -= 1.0;
                    1.0 + 0.5 * rng.signed()
                } else {
                    0.0
                };
                let envelope = (t * 6.0).min(1.0) * (1.0 - t);
                out[start + i] += 0.5 * envelope * body.process(slip);
            }
        }
        _ => {}
    }
}

/// The rest of setting a block down: it scoots a little into place, rubbing
/// on what's under it (gritty on stone and gravel, a brush of leaves), then
/// rocks and settles with a second, softer knock 15 to 40 ms after; sand and
/// gravel trickle after it.
fn settle_in(rng: &mut Rng, out: &mut Vec<f32>, material: Material, props: &Props, speed: f32) {
    let rate = SAMPLE_RATE as f32;
    let (rub, centre, seconds) = match material {
        Material::Stone | Material::Granular => (0.08, 3000.0, 0.04),
        Material::Wood => (0.06, 1800.0, 0.04),
        Material::Metal | Material::Glass | Material::Ice => (0.04, 4500.0, 0.03),
        Material::Earth => (0.05, 1200.0, 0.04),
        Material::Plant => (0.1, 3500.0, 0.1),
        Material::Soft | Material::Snow => (0.03, 900.0, 0.05),
    };
    let start = (0.004 * rate) as usize;
    let centre = centre * (0.8 + 0.4 * rng.unit());
    noise_burst(rng, out, start, seconds, centre, rub);
    let mut bump = strike(rng, Contact::Place, material, props, 0.3 * speed);
    let gain = 0.25 + 0.2 * rng.unit();
    bump.iter_mut().for_each(|x| *x *= gain);
    let at = ((0.015 + 0.025 * rng.unit()) * rate) as usize;
    if out.len() < at + bump.len() {
        out.resize(at + bump.len(), 0.0);
    }
    for (o, b) in out[at..].iter_mut().zip(&bump) {
        *o += b;
    }
    if material == Material::Granular {
        grains(rng, out, 15, 0.2, 0.08, 0.6);
    }
}

/// One strike of `contact` on `material` at `speed`: the surface's modes,
/// the contact's click, and what the material does besides (grains, crunch).
fn strike(
    rng: &mut Rng,
    contact: Contact,
    material: Material,
    props: &Props,
    speed: f32,
) -> Vec<f32> {
    let target = (props.young, props.poisson);
    let (mass, radius, striker) = match contact {
        Contact::Tap => (0.5, 0.01, (200e9, 0.3)),
        Contact::Place => (2.0, 0.05, target),
        Contact::Land => (70.0, 0.1, (10e6, 0.45)),
        Contact::Step => (8.0, 0.05, (10e6, 0.45)),
    };
    // Grains crushed go with the blow's momentum, against a landing's.
    let crush = (mass * speed / 350.0).sqrt().clamp(0.1, 1.5);
    let stiffness = effective_modulus(striker, target);
    let tau = hertz_time(mass, radius, stiffness, speed);
    let mut out = Vec::new();
    let mut bank = struck_modes(rng, props, tau, 1.0);
    bank.render_into(&mut out, 0);
    // The contact's own click: the force pulse's change, a half-sine's
    // derivative, as loud as the modes' start.
    let rate = SAMPLE_RATE as f32;
    let pulse = ((tau * rate) as usize).max(2);
    if out.len() < pulse {
        out.resize(pulse, 0.0);
    }
    for (i, sample) in out[..pulse].iter_mut().enumerate() {
        *sample += 0.6 * (PI * i as f32 / pulse as f32).cos();
    }
    // A long contact's click is dull: low-pass it (but not the grains,
    // which crunch on their own).
    let mut smooth = LowPass::new((1.0 / tau).clamp(80.0, 20_000.0) * 2.0);
    for sample in &mut out {
        *sample = smooth.process(*sample);
    }
    match material {
        Material::Granular => grains(rng, &mut out, (60.0 * crush) as usize, 0.06, 0.3, 0.7),
        Material::Earth => grains(rng, &mut out, (16.0 * crush) as usize, 0.04, 0.2, 0.2),
        Material::Snow => {
            // Bonds breaking as it compacts: crackling bursts, sizes ∝ E^-1.7,
            // over the 0.1 s the foot takes to sink in.
            // In snow, the crunch is most of what's heard.
            for _ in 0..((120.0 * crush) as usize).max(12) {
                let at = (0.1 * rng.unit().sqrt() * SAMPLE_RATE as f32) as usize;
                let energy = (1.0 + rng.unit() * (30f32.powf(-0.7) - 1.0)).powf(-1.0 / 0.7);
                let centre = 1500.0 + 2500.0 * rng.unit();
                noise_burst(rng, &mut out, at, 0.003, centre, 0.8 * energy.sqrt());
            }
        }
        Material::Plant => noise_burst(rng, &mut out, 0, 0.08, 4000.0, 0.6),
        _ => {}
    }
    out
}

/// A block of `material` breaking (M4). Brittle ones (glass, ice, stone)
/// crack, then shatter into fragments whose sizes follow N(>L) ∝ L⁻²
/// (brittle fragmentation), each ringing with its own small modes, then
/// falling 0.2 to 0.8 m and bouncing (each bounce √e as loud and e as long
/// to the next, for restitution e). Wood cracks with crackling noise and a
/// few splinters; earth and gravel crumble in grains; plants snap and
/// rustle; metal and soft stuff just give a dull blow.
pub fn fracture(material: Material, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let props = material.props();
    let rate = SAMPLE_RATE as f32;
    let mut out = Vec::new();
    // The crack: a burst of broadband noise as the block gives way.
    let (crack, centre) = match material {
        Material::Glass | Material::Ice => (0.012, 5000.0),
        Material::Stone => (0.02, 2500.0),
        Material::Wood => (0.1, 1500.0),
        Material::Plant => (0.03, 3000.0),
        _ => (0.03, 600.0),
    };
    noise_burst(&mut rng, &mut out, 0, crack, centre, 0.5);
    if material == Material::Wood {
        // Fibres giving way one after another: crackling, sizes ∝ E^-1.7.
        for _ in 0..40 {
            let at = (0.12 * rng.unit() * rate) as usize;
            let energy = (1.0 + rng.unit() * (50f32.powf(-0.7) - 1.0)).powf(-1.0 / 0.7);
            noise_burst(&mut rng, &mut out, at, 0.002, 2500.0, 0.05 * energy.sqrt());
        }
    }
    match material {
        Material::Earth => grains(&mut rng, &mut out, 150, 0.25, 0.15, 0.25),
        Material::Granular => grains(&mut rng, &mut out, 300, 0.35, 0.2, 0.7),
        Material::Plant => noise_burst(&mut rng, &mut out, 0, 0.18, 4500.0, 0.4),
        Material::Snow => grains(&mut rng, &mut out, 80, 0.2, 0.1, 0.4),
        Material::Metal | Material::Soft => {
            // Coming apart, it's no longer held to ring: five times the damping.
            let loose = Props {
                loss: props.loss * 5.0,
                ..props
            };
            let mut bank = struck_modes(&mut rng, &loose, 0.004, 0.8);
            bank.render_into(&mut out, 0);
        }
        _ => {}
    }
    for _ in 0..props.fragments {
        // Sizes from 2 cm, N(>L) ∝ L⁻², capped at 30 cm.
        let size = (0.02 * (1.0 - rng.unit()).powf(-0.5)).min(0.3);
        let amplitude = 0.15 * (size / 0.05).sqrt();
        // It rings as it breaks free...
        let mut bank = fragment_modes(&mut rng, &props, size, amplitude);
        bank.render_into(&mut out, (0.01 * rng.unit() * rate) as usize);
        // ...then lands and bounces.
        let drop = 0.2 + 0.6 * rng.unit();
        let landing = (2.0 * drop / GRAVITY).sqrt();
        let mut bounce_speed = (2.0 * GRAVITY * drop).sqrt();
        let mut at = landing;
        let mut loud = amplitude;
        while loud > 0.02 * amplitude && at < 1.5 {
            let mut bank = fragment_modes(&mut rng, &props, size, loud);
            bank.render_into(&mut out, (at * rate) as usize);
            // The landing's own click: all there is of the smallest chips,
            // whose modes ring above hearing.
            let ring = fragment_frequency(&props, size).min(8000.0);
            noise_burst(
                &mut rng,
                &mut out,
                (at * rate) as usize,
                0.001,
                ring,
                0.6 * loud,
            );
            bounce_speed *= props.restitution;
            at += 2.0 * bounce_speed / GRAVITY;
            loud *= props.restitution.sqrt();
        }
    }
    normalize(&mut out, 0.8 * loudness(&props).max(0.5));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crossings(s: &[f32]) -> f32 {
        s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count() as f32 / s.len() as f32
    }

    #[test]
    fn hertz_contact_times_are_physical() {
        let steel = (200e9, 0.3);
        let stone = Material::Stone.props();
        let earth = Material::Earth.props();
        let on_stone = hertz_time(
            0.5,
            0.01,
            effective_modulus(steel, (stone.young, stone.poisson)),
            3.0,
        );
        let on_earth = hertz_time(
            0.5,
            0.01,
            effective_modulus(steel, (earth.young, earth.poisson)),
            3.0,
        );
        // A pickaxe on rock: a fraction of a millisecond; on soil, several.
        assert!((0.000_1..0.001).contains(&on_stone), "{on_stone}");
        assert!(on_earth > 10.0 * on_stone, "{on_earth}");
        // A body landing on shoes: tens of milliseconds.
        let land = hertz_time(
            70.0,
            0.1,
            effective_modulus((10e6, 0.45), (stone.young, stone.poisson)),
            5.0,
        );
        assert!((0.01..0.06).contains(&land), "{land}");
    }

    #[test]
    fn plate_modes_follow_the_material() {
        let metal = Material::Metal.props();
        let f11 = metal.plate_mode(1.0, 1.0, 1.0, 1.0);
        assert!((200.0..300.0).contains(&f11), "{f11}");
        // Mode (2, 1) is 2.5 times (1, 1) on a square plate.
        assert!((metal.plate_mode(2.0, 1.0, 1.0, 1.0) / f11 - 2.5).abs() < 1e-3);
    }

    #[test]
    fn metal_rings_glass_is_bright_and_earth_thuds() {
        let tap = |m| contact(Contact::Tap, m, 3.0, 1);
        let (metal, stone, earth, glass) = (
            tap(Material::Metal),
            tap(Material::Stone),
            tap(Material::Earth),
            tap(Material::Glass),
        );
        assert!(
            metal.len() > 3 * stone.len(),
            "metal {} stone {}",
            metal.len(),
            stone.len()
        );
        let start = |s: &[f32]| s[..s.len().min(2400)].to_vec();
        assert!(crossings(&start(&glass)) > crossings(&start(&earth)));
        // Soft stuff is quieter struck.
        let peak = |s: &[f32]| s.iter().fold(0f32, |p, x| p.max(x.abs()));
        assert!(peak(&earth) < peak(&stone));
        // Landing is duller than tapping.
        let land = contact(Contact::Land, Material::Stone, 5.0, 1);
        assert!(crossings(&start(&land)) < crossings(&start(&stone)));
        for m in Material::ALL {
            for c in Contact::ALL {
                let s = contact(c, m, 3.0, 2);
                assert!(
                    !s.is_empty()
                        && s.len() < 5 * SAMPLE_RATE as usize
                        && s.iter().all(|x| x.is_finite())
                );
            }
        }
    }

    #[test]
    fn steps_are_soft_and_snow_crunches() {
        let peak = |s: &[f32]| s.iter().fold(0f32, |p, x| p.max(x.abs()));
        let step = contact(Contact::Step, Material::Stone, 1.0, 1);
        let tap = contact(Contact::Tap, Material::Stone, 3.0, 1);
        // The heel lands duller than a pick: its first 8 ms, before the sole scuffs.
        assert!(crossings(&step[..400]) < crossings(&tap[..400]));
        // Snow keeps crunching after the heel lands.
        let snow = contact(Contact::Step, Material::Snow, 1.0, 1);
        assert!(snow.len() > SAMPLE_RATE as usize / 20);
        assert!(peak(&snow[SAMPLE_RATE as usize / 50..]) > 0.3 * peak(&snow));
    }

    #[test]
    fn glass_shatters_and_tinkles_on() {
        let rate = SAMPLE_RATE as usize;
        let glass = fracture(Material::Glass, 3);
        // Shards are still landing and bouncing well after the break.
        let late = glass[rate / 4..rate / 2]
            .iter()
            .fold(0f32, |p, x| p.max(x.abs()));
        assert!(late > 0.05, "{late}");
        let earth = fracture(Material::Earth, 3);
        assert!(earth.len() < glass.len());
        for m in Material::ALL {
            let s = fracture(m, 4);
            assert!(
                !s.is_empty() && s.len() < 3 * rate && s.iter().all(|x| x.is_finite()),
                "{m:?}: {} samples, finite {}",
                s.len(),
                s.iter().all(|x| x.is_finite())
            );
        }
    }
}
