#[allow(unused_imports)] // Some of these are in core; the rest come from std.
use foley_math::Float as _;
/// ISO 9613-1 atmospheric absorption at `hz`, in dB a metre, in air at
/// `celsius` and `humidity` percent, at sea-level pressure. Dry air soaks up
/// treble faster than humid air. (The same formula as Earshot's.)
pub fn absorption_db_per_m(hz: f32, celsius: f32, humidity: f32) -> f32 {
    let (hz, celsius, humidity) = (f64::from(hz), f64::from(celsius), f64::from(humidity));
    let t = celsius + 273.15;
    let tr = t / 293.15;
    let saturation = 10f64.powf(-6.8346 * (273.16 / t).powf(1.261) + 4.6151);
    let h = humidity * saturation;
    let fr_o = 24.0 + 4.04e4 * h * (0.02 + h) / (0.391 + h);
    let fr_n = tr.powf(-0.5) * (9.0 + 280.0 * h * (-4.170 * (tr.powf(-1.0 / 3.0) - 1.0)).exp());
    let f2 = hz * hz;
    (8.686
        * f2
        * (1.84e-11 * tr.sqrt()
            + tr.powf(-2.5)
                * (0.01275 * (-2239.1 / t).exp() / (fr_o + f2 / fr_o)
                    + 0.1068 * (-3352.0 / t).exp() / (fr_n + f2 / fr_n)))) as f32
}

/// The frequency above which `distance` metres of air take more than 6 dB,
/// or `None` if it leaves everything up to 20 kHz.
pub fn absorption_cutoff(distance: f32, celsius: f32, humidity: f32) -> Option<f32> {
    let loss = |hz: f32| absorption_db_per_m(hz, celsius, humidity) * distance;
    if loss(20_000.0) < 6.0 {
        return None;
    }
    // Absorption rises with frequency: bisect in log frequency.
    let (mut lo, mut hi) = (20f32.ln(), 20_000f32.ln());
    for _ in 0..24 {
        let mid = 0.5 * (lo + hi);
        if loss(mid.exp()) < 6.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(lo.exp())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use std::{vec, vec::Vec};

    #[test]
    fn it_matches_iso_9613_1() {
        // ISO 9613-2's table at 20 °C and 70 % humidity: 5.0 dB/km at 1 kHz,
        // 22.9 at 4 kHz and 76.6 at 8 kHz.
        let at = |hz| absorption_db_per_m(hz, 20.0, 70.0) * 1000.0;
        assert!((at(1000.0) - 5.0).abs() < 0.2, "{}", at(1000.0));
        assert!((at(4000.0) - 22.9).abs() < 0.5, "{}", at(4000.0));
        assert!((at(8000.0) - 76.6).abs() < 1.5, "{}", at(8000.0));
        // A kilometre takes the top off, and further takes more.
        let near = absorption_cutoff(1000.0, 20.0, 70.0).expect("cut");
        let far = absorption_cutoff(5000.0, 20.0, 70.0).expect("cut");
        assert!(
            far < near && (500.0..2000.0).contains(&near),
            "{near} {far}"
        );
        assert_eq!(absorption_cutoff(10.0, 20.0, 70.0), None);
    }
}
