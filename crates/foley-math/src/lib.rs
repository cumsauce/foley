//! The float functions `foley-dsp` needs, which `core` lacks: std's own,
//! as methods, so a `no_std` crate calls them as it would with std
//! (`x.sin()`) and gets the same code. Only math lives here, so depending on
//! it can't bring in an allocation. To try a faster approximation of one,
//! change it here.

/// Std's float functions, as methods.
pub trait Float: Copy {
    fn sin(self) -> Self;
    fn cos(self) -> Self;
    fn tan(self) -> Self;
    fn asin(self) -> Self;
    fn acos(self) -> Self;
    fn atan(self) -> Self;
    fn sinh(self) -> Self;
    fn cosh(self) -> Self;
    fn tanh(self) -> Self;
    fn exp(self) -> Self;
    fn exp2(self) -> Self;
    fn exp_m1(self) -> Self;
    fn ln(self) -> Self;
    fn ln_1p(self) -> Self;
    fn log2(self) -> Self;
    fn log10(self) -> Self;
    fn sqrt(self) -> Self;
    fn cbrt(self) -> Self;
    fn floor(self) -> Self;
    fn ceil(self) -> Self;
    fn round(self) -> Self;
    fn trunc(self) -> Self;
    fn fract(self) -> Self;
    fn signum(self) -> Self;
    fn powf(self, other: Self) -> Self;
    fn atan2(self, other: Self) -> Self;
    fn hypot(self, other: Self) -> Self;
    fn rem_euclid(self, other: Self) -> Self;
    fn log(self, other: Self) -> Self;
    fn powi(self, n: i32) -> Self;
    fn mul_add(self, a: Self, b: Self) -> Self;
}

impl Float for f32 {
    #[inline]
    fn sin(self) -> Self {
        f32::sin(self)
    }

    #[inline]
    fn cos(self) -> Self {
        f32::cos(self)
    }

    #[inline]
    fn tan(self) -> Self {
        f32::tan(self)
    }

    #[inline]
    fn asin(self) -> Self {
        f32::asin(self)
    }

    #[inline]
    fn acos(self) -> Self {
        f32::acos(self)
    }

    #[inline]
    fn atan(self) -> Self {
        f32::atan(self)
    }

    #[inline]
    fn sinh(self) -> Self {
        f32::sinh(self)
    }

    #[inline]
    fn cosh(self) -> Self {
        f32::cosh(self)
    }

    #[inline]
    fn tanh(self) -> Self {
        f32::tanh(self)
    }

    #[inline]
    fn exp(self) -> Self {
        f32::exp(self)
    }

    #[inline]
    fn exp2(self) -> Self {
        f32::exp2(self)
    }

    #[inline]
    fn exp_m1(self) -> Self {
        f32::exp_m1(self)
    }

    #[inline]
    fn ln(self) -> Self {
        f32::ln(self)
    }

    #[inline]
    fn ln_1p(self) -> Self {
        f32::ln_1p(self)
    }

    #[inline]
    fn log2(self) -> Self {
        f32::log2(self)
    }

    #[inline]
    fn log10(self) -> Self {
        f32::log10(self)
    }

    #[inline]
    fn sqrt(self) -> Self {
        f32::sqrt(self)
    }

    #[inline]
    fn cbrt(self) -> Self {
        f32::cbrt(self)
    }

    #[inline]
    fn floor(self) -> Self {
        f32::floor(self)
    }

    #[inline]
    fn ceil(self) -> Self {
        f32::ceil(self)
    }

    #[inline]
    fn round(self) -> Self {
        f32::round(self)
    }

    #[inline]
    fn trunc(self) -> Self {
        f32::trunc(self)
    }

    #[inline]
    fn fract(self) -> Self {
        f32::fract(self)
    }

    #[inline]
    fn signum(self) -> Self {
        f32::signum(self)
    }

    #[inline]
    fn powf(self, other: Self) -> Self {
        f32::powf(self, other)
    }

    #[inline]
    fn atan2(self, other: Self) -> Self {
        f32::atan2(self, other)
    }

    #[inline]
    fn hypot(self, other: Self) -> Self {
        f32::hypot(self, other)
    }

    #[inline]
    fn rem_euclid(self, other: Self) -> Self {
        f32::rem_euclid(self, other)
    }

    #[inline]
    fn log(self, other: Self) -> Self {
        f32::log(self, other)
    }

    #[inline]
    fn powi(self, n: i32) -> Self {
        f32::powi(self, n)
    }

    #[inline]
    fn mul_add(self, a: Self, b: Self) -> Self {
        f32::mul_add(self, a, b)
    }
}
impl Float for f64 {
    #[inline]
    fn sin(self) -> Self {
        f64::sin(self)
    }

    #[inline]
    fn cos(self) -> Self {
        f64::cos(self)
    }

    #[inline]
    fn tan(self) -> Self {
        f64::tan(self)
    }

    #[inline]
    fn asin(self) -> Self {
        f64::asin(self)
    }

    #[inline]
    fn acos(self) -> Self {
        f64::acos(self)
    }

    #[inline]
    fn atan(self) -> Self {
        f64::atan(self)
    }

    #[inline]
    fn sinh(self) -> Self {
        f64::sinh(self)
    }

    #[inline]
    fn cosh(self) -> Self {
        f64::cosh(self)
    }

    #[inline]
    fn tanh(self) -> Self {
        f64::tanh(self)
    }

    #[inline]
    fn exp(self) -> Self {
        f64::exp(self)
    }

    #[inline]
    fn exp2(self) -> Self {
        f64::exp2(self)
    }

    #[inline]
    fn exp_m1(self) -> Self {
        f64::exp_m1(self)
    }

    #[inline]
    fn ln(self) -> Self {
        f64::ln(self)
    }

    #[inline]
    fn ln_1p(self) -> Self {
        f64::ln_1p(self)
    }

    #[inline]
    fn log2(self) -> Self {
        f64::log2(self)
    }

    #[inline]
    fn log10(self) -> Self {
        f64::log10(self)
    }

    #[inline]
    fn sqrt(self) -> Self {
        f64::sqrt(self)
    }

    #[inline]
    fn cbrt(self) -> Self {
        f64::cbrt(self)
    }

    #[inline]
    fn floor(self) -> Self {
        f64::floor(self)
    }

    #[inline]
    fn ceil(self) -> Self {
        f64::ceil(self)
    }

    #[inline]
    fn round(self) -> Self {
        f64::round(self)
    }

    #[inline]
    fn trunc(self) -> Self {
        f64::trunc(self)
    }

    #[inline]
    fn fract(self) -> Self {
        f64::fract(self)
    }

    #[inline]
    fn signum(self) -> Self {
        f64::signum(self)
    }

    #[inline]
    fn powf(self, other: Self) -> Self {
        f64::powf(self, other)
    }

    #[inline]
    fn atan2(self, other: Self) -> Self {
        f64::atan2(self, other)
    }

    #[inline]
    fn hypot(self, other: Self) -> Self {
        f64::hypot(self, other)
    }

    #[inline]
    fn rem_euclid(self, other: Self) -> Self {
        f64::rem_euclid(self, other)
    }

    #[inline]
    fn log(self, other: Self) -> Self {
        f64::log(self, other)
    }

    #[inline]
    fn powi(self, n: i32) -> Self {
        f64::powi(self, n)
    }

    #[inline]
    fn mul_add(self, a: Self, b: Self) -> Self {
        f64::mul_add(self, a, b)
    }
}
