/// A small, fast generator (xorshift64*): synthesis needs many cheap random
/// numbers, not cryptographic ones.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Uniform in [-1, 1).
    pub fn signed(&mut self) -> f32 {
        2.0 * self.unit() - 1.0
    }

    /// The wait to the next event of a Poisson process at `rate` a second.
    pub fn exponential(&mut self, rate: f32) -> f32 {
        -(1.0 - self.unit()).ln() / rate.max(1e-6)
    }
}
