//! Seeded randomness. Every draw in a show comes from an `Rng` passed down
//! explicitly, so a seed replays a show exactly and engines never share
//! state.

use crate::particle::Vec3;

/// A 64-bit linear congruential generator (Knuth's MMIX constants), seeded
/// through SplitMix64 so neighbouring seeds give unrelated sequences.
pub struct Rng {
    state: u64,
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self {
            state: splitmix(seed),
        }
    }

    /// An independent stream for a numbered purpose (a loop of the show).
    pub fn derive(seed: u64, stream: u64) -> Self {
        Self::new(splitmix(seed) ^ splitmix(stream.wrapping_add(0x5EED)))
    }

    /// Uniform in [0, 1).
    pub fn next(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn range(&mut self, min: f64, max: f64) -> f64 {
        self.next() * (max - min) + min
    }

    pub fn chance(&mut self, probability: f64) -> bool {
        self.next() < probability
    }

    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[((self.next() * items.len() as f64) as usize).min(items.len() - 1)]
    }

    /// Standard normal deviate (Box–Muller).
    pub fn gauss(&mut self) -> f64 {
        let u = self.next().max(1e-12);
        let v = self.next();
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }

    /// Uniformly distributed direction on the unit sphere.
    pub fn unit(&mut self) -> Vec3 {
        let z = self.range(-1.0, 1.0);
        let a = self.range(0.0, std::f64::consts::TAU);
        let r = (1.0 - z * z).sqrt();
        Vec3::new(r * a.cos(), r * a.sin(), z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seed_replays_and_streams_differ() {
        let draw = |mut rng: Rng| (0..100).map(|_| rng.next()).collect::<Vec<_>>();
        assert_eq!(draw(Rng::new(7)), draw(Rng::new(7)));
        assert_ne!(draw(Rng::new(7)), draw(Rng::new(8)));
        assert_ne!(draw(Rng::derive(7, 0)), draw(Rng::derive(7, 1)));
        let mut rng = Rng::new(1);
        let mean = (0..20_000).map(|_| rng.next()).sum::<f64>() / 20_000.0;
        assert!((mean - 0.5).abs() < 0.01);
        assert!((0..1000).all(|_| (rng.unit().length() - 1.0).abs() < 1e-9));
    }
}
