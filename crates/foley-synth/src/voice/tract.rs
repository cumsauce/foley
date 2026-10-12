//! A Kelly–Lochbaum digital waveguide vocal tract with a nasal branch, in
//! pressure waves: sections 3.6 mm long (sound's travel in half a sample),
//! two steps a sample, scattering where their areas change, a three-way
//! junction at the velum, losses along the walls, and reflections at the
//! glottis (0.75) and at the lips and nostrils (−0.85).

use crate::SAMPLE_RATE;

const SPEED_OF_SOUND: f32 = 343.0;
const GLOTTIS: f32 = 0.75;
const LIPS: f32 = -0.85;
const NOSTRILS: f32 = -0.85;
const FADE: f32 = 0.999;

/// How long a section is, m.
pub(super) fn section_length() -> f32 {
    SPEED_OF_SOUND / (2.0 * SAMPLE_RATE as f32)
}

#[derive(Debug, Clone)]
struct Tube {
    right: Vec<f32>,
    left: Vec<f32>,
    out_right: Vec<f32>,
    out_left: Vec<f32>,
    area: Vec<f32>,
}

impl Tube {
    fn new(n: usize) -> Tube {
        Tube {
            right: vec![0.0; n],
            left: vec![0.0; n],
            out_right: vec![0.0; n + 1],
            out_left: vec![0.0; n + 1],
            area: vec![1.0; n],
        }
    }

    fn len(&self) -> usize {
        self.right.len()
    }

    /// Two-port junctions between every pair of sections from `from`, as
    /// pressure: p = 2(A₁p₁⁺ + A₂p₂⁻)/(A₁ + A₂), each outgoing wave p minus
    /// what came in on that side.
    fn scatter(&mut self, from: usize, skip: Option<usize>) {
        for i in from.max(1)..self.len() {
            if Some(i) == skip {
                continue;
            }
            let (a1, a2) = (self.area[i - 1], self.area[i]);
            let p = 2.0 * (a1 * self.right[i - 1] + a2 * self.left[i]) / (a1 + a2);
            self.out_right[i] = p - self.left[i];
            self.out_left[i] = p - self.right[i - 1];
        }
    }

    fn propagate(&mut self) {
        let n = self.len();
        for i in 0..n {
            self.right[i] = self.out_right[i] * FADE;
            self.left[i] = self.out_left[i + 1] * FADE;
        }
    }
}

/// The tract, its shape set from articulation by [`Tract::shape`].
#[derive(Debug, Clone)]
pub(super) struct Tract {
    oral: Tube,
    nose: Tube,
    velum_at: usize,
    /// Diameters (cm, human scale; only ratios matter) aimed for, and how
    /// fast each moves there.
    target: Vec<f32>,
    diameter: Vec<f32>,
    velum_target: f32,
    velum: f32,
    rest: Vec<f32>,
}

impl Tract {
    /// A tract `length` m long, with a nose.
    pub(super) fn new(length: f32) -> Tract {
        let n = ((length / section_length()).round() as usize).clamp(8, 180);
        let rest: Vec<f32> = (0..n)
            .map(|i| {
                let x = i as f32 / n as f32;
                if x < 7.0 / 44.0 {
                    0.6
                } else if x < 12.0 / 44.0 {
                    1.1
                } else {
                    1.5
                }
            })
            .collect();
        let nose_n = ((n as f32) * 28.0 / 44.0).round().max(4.0) as usize;
        let mut nose = Tube::new(nose_n);
        for i in 0..nose_n {
            let d = 2.0 * i as f32 / nose_n as f32;
            let d = if d < 1.0 {
                0.4 + 1.6 * d
            } else {
                0.5 + 1.5 * (2.0 - d)
            };
            nose.area[i] = d.min(1.9).powi(2);
        }
        let mut tract = Tract {
            oral: Tube::new(n),
            nose,
            velum_at: n - nose_n + 1,
            target: rest.clone(),
            diameter: rest.clone(),
            velum_target: 0.01,
            velum: 0.01,
            rest,
        };
        tract.update_areas();
        tract
    }

    /// Sets the shape to aim for. The tongue's body (Pink Trombone's
    /// model): `tongue` along the tract (0 glottis to 1 lips; vowels from
    /// about 0.27, back, to 0.66, front) and its `height` as Pink Trombone's
    /// tongue diameter (2.05 close to 3.5 open); `lips` open 0 to 1; the
    /// `velum` 0 shut to 1 open; and a `constriction` (place 0 to 1, diameter
    /// cm), for frication or a closure.
    pub(super) fn shape(
        &mut self,
        tongue: f32,
        height: f32,
        lips: f32,
        velum: f32,
        constriction: Option<(f32, f32)>,
    ) {
        let n = self.oral.len();
        let at = |x: f32| (x * n as f32).round() as isize;
        let blade = at(10.0 / 44.0);
        let tip = at(32.0 / 44.0);
        let lip = at(39.0 / 44.0);
        let index = tongue.clamp(0.0, 1.0) * n as f32;
        let fixed = 2.0 + (height.clamp(2.0, 3.6) - 2.0) / 1.5;
        for i in 0..n {
            let ii = i as isize;
            let mut d = self.rest[i];
            if ii >= blade && ii < lip {
                let t = 1.1 * std::f32::consts::PI * (index - i as f32) / (tip - blade) as f32;
                let mut curve = (1.5 - fixed + 1.7) * t.cos();
                if ii == blade - 2 || ii == lip - 1 {
                    curve *= 0.8;
                }
                if ii == blade || ii == lip - 2 {
                    curve *= 0.94;
                }
                d = self.rest[i] - curve;
            }
            if ii >= lip {
                d = 1.5 * lips.clamp(0.0, 1.0);
            }
            if let Some((place, width)) = constriction {
                let c = place * n as f32;
                let reach = (n as f32 * 0.12).max(2.0);
                let dist = (i as f32 - c).abs();
                if dist < reach {
                    let blend = (dist / reach).powi(2);
                    d = d.min(width + blend * (d - width).max(0.0));
                }
            }
            self.target[i] = d.max(0.0);
        }
        self.velum_target = 0.01 + 0.39 * velum.clamp(0.0, 1.0);
    }

    /// Moves a step towards the target shape: a few milliseconds to get
    /// there.
    pub(super) fn settle(&mut self, rate: f32) {
        for (d, t) in self.diameter.iter_mut().zip(&self.target) {
            *d += (t - *d) * rate;
        }
        self.velum += (self.velum_target - self.velum) * rate;
        self.update_areas();
    }

    fn update_areas(&mut self) {
        for (a, d) in self.oral.area.iter_mut().zip(&self.diameter) {
            *a = (d * d).max(1e-4);
        }
        self.nose.area[0] = self.velum.max(1e-4);
    }

    /// Where along it a constriction at `place` (0 to 1) is, as a section.
    pub(super) fn index_of(&self, place: f32) -> usize {
        ((place.clamp(0.0, 1.0) * self.oral.len() as f32) as usize).min(self.oral.len() - 1)
    }

    /// One sample: `glottis` drives the lower end, `noise` (section,
    /// pressure) is turbulence at a constriction. Returns what radiates from
    /// the lips and nostrils.
    pub(super) fn run(&mut self, glottis: f32, noise: Option<(usize, f32)>) -> f32 {
        let mut out = 0.0;
        for _ in 0..2 {
            out += self.step(glottis, noise);
        }
        0.5 * out
    }

    fn step(&mut self, glottis: f32, noise: Option<(usize, f32)>) -> f32 {
        let n = self.oral.len();
        let v = self.velum_at;
        self.oral.out_right[0] = self.oral.left[0] * GLOTTIS + glottis;
        self.oral.out_left[n] = self.oral.right[n - 1] * LIPS;
        self.oral.scatter(1, Some(v));
        // The velum: three tubes meet.
        let (al, ar, an) = (self.oral.area[v - 1], self.oral.area[v], self.nose.area[0]);
        let p = 2.0
            * (al * self.oral.right[v - 1] + ar * self.oral.left[v] + an * self.nose.left[0])
            / (al + ar + an);
        self.oral.out_right[v] = p - self.oral.left[v];
        self.oral.out_left[v] = p - self.oral.right[v - 1];
        self.nose.out_right[0] = p - self.nose.left[0];
        if let Some((i, pressure)) = noise {
            let i = i.min(n - 1);
            self.oral.out_right[i] += 0.5 * pressure;
            self.oral.out_left[i] += 0.5 * pressure;
        }
        self.oral.propagate();
        let m = self.nose.len();
        self.nose.out_left[m] = self.nose.right[m - 1] * NOSTRILS;
        self.nose.scatter(1, None);
        self.nose.propagate();
        self.oral.right[n - 1] * (1.0 + LIPS) + self.nose.right[m - 1] * (1.0 + NOSTRILS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The lowest resonance of a tract driven by an impulse, by counting
    /// zero crossings of its ringing.
    fn ring(length: f32) -> f32 {
        let mut tract = Tract::new(length);
        // A uniform open tube.
        tract.target.iter_mut().for_each(|d| *d = 1.5);
        for _ in 0..2000 {
            tract.settle(0.05);
        }
        let mut out = Vec::new();
        for i in 0..4800 {
            out.push(tract.run(if i == 0 { 1.0 } else { 0.0 }, None));
        }
        // The strongest response between 80 and 1000 Hz: the first formant.
        let power = |f: f32| {
            let (mut re, mut im) = (0.0f32, 0.0f32);
            for (n, x) in out.iter().enumerate() {
                let w = std::f32::consts::TAU * f * n as f32 / SAMPLE_RATE as f32;
                re += x * w.cos();
                im += x * w.sin();
            }
            re * re + im * im
        };
        (8..100)
            .map(|i| i as f32 * 10.0)
            .fold((0.0, 0.0), |best: (f32, f32), f| {
                let p = power(f);
                if p > best.1 { (f, p) } else { best }
            })
            .0
    }

    #[test]
    fn a_tube_resonates_at_a_quarter_wavelength() {
        // c / 4L: 504 Hz for 17 cm, 191 Hz for 45 cm.
        let human = ring(0.17);
        assert!((human - 504.0).abs() < 80.0, "{human} Hz");
        let cow = ring(0.45);
        assert!((cow - 191.0).abs() < 40.0, "{cow} Hz");
    }

    #[test]
    fn it_stays_stable_when_shut() {
        let mut tract = Tract::new(0.17);
        tract.shape(0.5, 2.05, 0.0, 1.0, Some((0.6, 0.0)));
        for _ in 0..500 {
            tract.settle(0.05);
        }
        let mut peak = 0f32;
        for i in 0..48_000 {
            let x = tract.run(if i % 200 == 0 { 1.0 } else { 0.0 }, None);
            peak = peak.max(x.abs());
            assert!(x.is_finite());
        }
        assert!(peak < 50.0, "{peak}");
    }
}

#[cfg(test)]
mod formants {
    use super::*;

    /// The first two spectral peaks of the tract shaped as given.
    pub(crate) fn measure(tongue: f32, height: f32, lips: f32) -> (f32, f32) {
        let mut tract = Tract::new(0.17);
        tract.shape(tongue, height, lips, 0.0, None);
        for _ in 0..3000 {
            tract.settle(0.05);
        }
        let out: Vec<f32> = (0..9600)
            .map(|i| tract.run(if i == 0 { 1.0 } else { 0.0 }, None))
            .collect();
        let power = |f: f32| {
            let (mut re, mut im) = (0.0f32, 0.0f32);
            for (n, x) in out.iter().enumerate() {
                let w = std::f32::consts::TAU * f * n as f32 / SAMPLE_RATE as f32;
                re += x * w.cos();
                im += x * w.sin();
            }
            re * re + im * im
        };
        let spectrum: Vec<(f32, f32)> = (10..400)
            .map(|i| (i as f32 * 10.0, power(i as f32 * 10.0)))
            .collect();
        let peaks: Vec<f32> = spectrum
            .windows(3)
            .filter(|w| w[1].1 > w[0].1 && w[1].1 > w[2].1)
            .map(|w| w[1].0)
            .collect();
        (
            peaks.first().copied().unwrap_or(0.0),
            peaks.get(1).copied().unwrap_or(0.0),
        )
    }

    #[test]
    fn vowels_have_their_formants() {
        // (tongue, height, lips) as score.rs has them, and Peterson and Barney's F1, F2.
        for (t, h, l, f1, f2) in [
            (0.64, 2.05, 0.8, 270.0, 2290.0),
            (0.58, 2.55, 0.9, 530.0, 1840.0),
            (0.53, 2.9, 1.0, 660.0, 1720.0),
            (0.35, 2.8, 0.85, 640.0, 1190.0),
            (0.27, 2.7, 1.0, 730.0, 1090.0),
        ] {
            let (m1, m2) = measure(t, h, l);
            assert!(
                (m1 / f1 - 1.0).abs() < 0.2 && (m2 / f2 - 1.0).abs() < 0.2,
                "{m1}/{m2} against {f1}/{f2}"
            );
        }
    }
}
