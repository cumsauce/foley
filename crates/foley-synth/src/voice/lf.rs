//! The Liljencrants–Fant glottal flow model, set by Fant's single Rd
//! parameter (1995): from pressed and creaky (0.3) through modal (about 1)
//! to breathy (2.7).

use std::f64::consts::PI;
use std::sync::OnceLock;

pub(super) const RD_MIN: f32 = 0.3;
pub(super) const RD_MAX: f32 = 2.7;
const STEPS: usize = 25;
const TABLE: usize = 1024;

/// One period of the LF model: the flow's derivative (what the glottis
/// excites the tract with, peak negative −1) and the flow (peak 1).
#[derive(Debug, Clone)]
pub struct Pulse {
    pub derivative: Vec<f32>,
    pub flow: Vec<f32>,
}

/// The LF pulse for `rd`. Fant's regressions give its timing (in periods):
/// Ra = (−1 + 4.8·Rd)/100, Rk = (22.4 + 11.8·Rd)/100, and Rg from
/// Rd = (1/0.11)(0.5 + 1.2·Rk)(Rk/4Rg + Ra); then Tp = 1/2Rg, Te = Tp(1 + Rk),
/// Ta = Ra. The open phase is E₀e^{αt}sin(πt/Tp); the return phase an
/// exponential that closes by the period's end. ε solves εTa = 1 − e^{−ε(1−Te)},
/// and α makes the net flow zero.
pub fn lf_pulse(rd: f32) -> Pulse {
    let rd = f64::from(rd.clamp(RD_MIN, RD_MAX));
    let ra = (-1.0 + 4.8 * rd) / 100.0;
    let rk = (22.4 + 11.8 * rd) / 100.0;
    let rg = rk / (4.0 * ((0.11 * rd) / (0.5 + 1.2 * rk) - ra));
    let tp = 1.0 / (2.0 * rg);
    let te = (tp * (1.0 + rk)).min(0.97);
    let ta = ra.max(0.002);
    let mut eps = 1.0 / ta;
    for _ in 0..60 {
        eps = (1.0 - (-eps * (1.0 - te)).exp()) / ta;
    }
    let wg = PI / tp;
    let closing = (-eps * (1.0 - te)).exp();
    let ret = -(1.0 / (eps * ta)) * ((1.0 - closing) / eps - (1.0 - te) * closing);
    // E(Te) = −1: E₀ = −1 / (e^{αTe} sin(ωTe)).
    let e0 = |a: f64| -1.0 / ((a * te).exp() * (wg * te).sin());
    let open = |a: f64| {
        e0(a)
            * (((a * te).exp() * (a * (wg * te).sin() - wg * (wg * te).cos()) + wg)
                / (a * a + wg * wg))
    };
    let net = |a: f64| open(a) + ret;
    // Bracket the root, then bisect.
    let (mut lo, mut hi) = (-20.0f64, 20.0f64);
    let mut a = -20.0;
    while a < 200.0 {
        if net(a).signum() != net(a + 0.5).signum() {
            lo = a;
            hi = a + 0.5;
            break;
        }
        a += 0.5;
    }
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if net(lo).signum() == net(mid).signum() {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let alpha = 0.5 * (lo + hi);
    let mut derivative: Vec<f32> = (0..TABLE)
        .map(|i| {
            let t = i as f64 / TABLE as f64;
            let v = if t < te {
                e0(alpha) * (alpha * t).exp() * (wg * t).sin()
            } else {
                -(1.0 / (eps * ta)) * ((-eps * (t - te)).exp() - closing)
            };
            v as f32
        })
        .collect();
    // Whatever net flow is left from sampling goes; peak negative to −1.
    let mean = derivative.iter().sum::<f32>() / TABLE as f32;
    derivative.iter_mut().for_each(|d| *d -= mean);
    let low = derivative
        .iter()
        .fold(0f32, |m, d| m.min(*d))
        .abs()
        .max(1e-6);
    derivative.iter_mut().for_each(|d| *d /= low);
    let mut flow = Vec::with_capacity(TABLE);
    let mut sum = 0.0f32;
    for d in &derivative {
        sum += d;
        flow.push(sum);
    }
    let base = flow.iter().fold(f32::MAX, |m, f| m.min(*f));
    let top = flow.iter().fold(f32::MIN, |m, f| m.max(*f)) - base;
    flow.iter_mut()
        .for_each(|f| *f = (*f - base) / top.max(1e-6));
    Pulse { derivative, flow }
}

fn tables() -> &'static [Pulse] {
    static TABLES: OnceLock<Vec<Pulse>> = OnceLock::new();
    TABLES.get_or_init(|| {
        (0..STEPS)
            .map(|i| lf_pulse(RD_MIN + (RD_MAX - RD_MIN) * i as f32 / (STEPS - 1) as f32))
            .collect()
    })
}

/// The flow derivative and flow at `phase` (0 to 1) of a period with `rd`,
/// interpolated between the tables.
pub(super) fn lookup(rd: f32, phase: f32) -> (f32, f32) {
    let tables = tables();
    let x = ((rd.clamp(RD_MIN, RD_MAX) - RD_MIN) / (RD_MAX - RD_MIN)) * (STEPS - 1) as f32;
    let i = (x as usize).min(STEPS - 2);
    let w = x - i as f32;
    let p = phase.rem_euclid(1.0) * TABLE as f32;
    let j = (p as usize).min(TABLE - 1);
    let k = (j + 1) % TABLE;
    let f = p - j as f32;
    let at = |t: &Pulse| {
        (
            t.derivative[j] + f * (t.derivative[k] - t.derivative[j]),
            t.flow[j] + f * (t.flow[k] - t.flow[j]),
        )
    };
    let (a, b) = (at(&tables[i]), at(&tables[i + 1]));
    (a.0 + w * (b.0 - a.0), a.1 + w * (b.1 - a.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulses_have_no_net_flow_and_breathier_ones_are_smoother() {
        for rd in [0.3, 1.0, 2.7] {
            let p = lf_pulse(rd);
            let net: f32 = p.derivative.iter().sum::<f32>() / p.derivative.len() as f32;
            assert!(net.abs() < 1e-4, "{rd}: {net}");
            assert!(p.derivative.iter().all(|d| d.is_finite()));
            assert!((p.derivative.iter().fold(0f32, |m, d| m.min(*d)) + 1.0).abs() < 1e-4);
        }
        // Breathy voice has less high-frequency energy: its derivative
        // changes less sharply.
        let sharp = |rd: f32| {
            let p = lf_pulse(rd);
            p.derivative
                .windows(2)
                .map(|w| (w[1] - w[0]).abs())
                .fold(0f32, f32::max)
        };
        assert!(sharp(2.7) < sharp(0.5));
    }
}
