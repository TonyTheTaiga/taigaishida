//! Fixed-size motion history: no per-frame allocations and no extra simulated sparks.
use crate::particle::Vec3;

pub struct Trail {
    positions: [Vec3; 8],
    count: usize,
    next: usize,
    ticks: usize,
}

impl Trail {
    pub fn new() -> Self {
        Self {
            positions: [Vec3::default(); 8],
            count: 0,
            next: 0,
            ticks: 0,
        }
    }

    pub fn record(&mut self, position: Vec3, interval: usize) {
        if self.ticks % interval == 0 {
            self.positions[self.next] = position;
            self.next = (self.next + 1) % self.positions.len();
            self.count = (self.count + 1).min(self.positions.len());
        }
        self.ticks += 1;
    }

    pub fn samples(&self) -> impl Iterator<Item = Vec3> + '_ {
        (0..self.count).map(|age| self.positions[(self.next + 7 - age) % 8])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_wraps_and_keeps_newest_samples_first() {
        let mut trail = Trail::new();
        for i in 0..40 {
            trail.record(Vec3::new(i as f64, 0.0, 0.0), 3);
        }
        let xs: Vec<_> = trail.samples().map(|p| p.x).collect();
        assert_eq!(xs, [39.0, 36.0, 33.0, 30.0, 27.0, 24.0, 21.0, 18.0]);
    }
}
