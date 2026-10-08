//! Everything in flight, and the one place it is advanced.
//!
//! The `World` owns the shells, stars, sparks, and smoke, and steps them on a
//! fixed clock. Anything burning reports what it creates through a single
//! `Spawn`, which also carries the world's seeded dice, so the step is the
//! same whether the engine or a test drives it.

use crate::particle::{Particles, Vec3};
use crate::rng::Rng;
use crate::shell::{Flight, Shell};
use crate::show::{Budget, Cue};
use crate::star::{Puff, Spark, Star};

/// Share of stars whose burnt-out smoke is tracked as its own parcel. Each
/// stands for the smoke of several stars, so a burst leaves a faint shell of
/// smoke the size of its flower.
const STAR_SMOKE_SHARE: f64 = 0.12;

/// Where anything burning sends what it creates this step. New stars and
/// shells join the world after the step; sparks and smoke join at once.
pub struct Spawn<'a> {
    pub rng: &'a mut Rng,
    pub stars: &'a mut Vec<Star>,
    pub shells: &'a mut Vec<Shell>,
    pub sparks: &'a mut Particles<Spark>,
    pub puffs: &'a mut Particles<Puff>,
    /// 0..=1 sampling rate that keeps the spark budget from saturating.
    pub spark_rate: f64,
}

pub struct World {
    pub rng: Rng,
    pub shells: Vec<Shell>,
    pub stars: Particles<Star>,
    pub sparks: Particles<Spark>,
    pub puffs: Particles<Puff>,
    new_stars: Vec<Star>,
    new_shells: Vec<Shell>,
    /// Where shells burst during the last step.
    pub bursts: Vec<Vec3>,
}

impl World {
    pub fn new(budget: &Budget, seed: u64) -> Self {
        Self {
            rng: Rng::new(seed),
            shells: Vec::new(),
            stars: Particles::new(budget.stars),
            sparks: Particles::new(budget.sparks),
            puffs: Particles::new(budget.puffs),
            new_stars: Vec::new(),
            new_shells: Vec::new(),
            bursts: Vec::new(),
        }
    }

    /// Fire a cue's device from its mortar.
    pub fn fire(&mut self, cue: &Cue) {
        cue.fire(&mut Spawn {
            rng: &mut self.rng,
            stars: &mut self.new_stars,
            shells: &mut self.new_shells,
            sparks: &mut self.sparks,
            puffs: &mut self.puffs,
            spark_rate: 1.0,
        });
        self.admit();
    }

    /// Put a shell in flight directly.
    #[cfg(test)]
    pub fn launch(&mut self, shell: Shell) {
        self.shells.push(shell);
    }

    /// Advance everything by one fixed step of `dt` seconds.
    pub fn step(&mut self, dt: f64) {
        self.bursts.clear();
        // Thin spark sampling once half the budget is in use, rather than
        // letting whichever star updates first claim every slot.
        let spark_rate = (self.sparks.headroom() * 2.0).min(1.0);
        let mut spawn = Spawn {
            rng: &mut self.rng,
            stars: &mut self.new_stars,
            shells: &mut self.new_shells,
            sparks: &mut self.sparks,
            puffs: &mut self.puffs,
            spark_rate,
        };

        let mut i = 0;
        while i < self.shells.len() {
            match self.shells[i].update(dt, &mut spawn) {
                Flight::Climbing => i += 1,
                Flight::Burst => {
                    let mut shell = self.shells.swap_remove(i);
                    self.bursts.push(shell.body.position);
                    shell.burst(&mut spawn);
                }
                Flight::Lost => {
                    self.shells.swap_remove(i);
                }
            }
        }

        let mut i = 0;
        while i < self.stars.items.len() {
            if self.stars.items[i].update(dt, &mut spawn) {
                i += 1;
                continue;
            }
            let star = self.stars.items.swap_remove(i);
            let p = star.body.position;
            if star.radius() <= 0.0 && p.y > 0.0 && spawn.rng.chance(STAR_SMOKE_SHARE) {
                let radius = spawn.rng.range(2.5, 4.5);
                let life = spawn.rng.range(14.0, 22.0);
                spawn.puffs.push(Puff::smoke(
                    p,
                    star.body.velocity.scale(0.3),
                    radius,
                    0.12,
                    life,
                ));
            }
        }

        let rng = &mut self.rng;
        self.sparks.items.retain_mut(|spark| spark.update(dt, rng));
        self.puffs.items.retain_mut(|puff| puff.update(dt));
        self.admit();
    }

    /// Bring in what was created during the last step or firing.
    fn admit(&mut self) {
        self.shells.append(&mut self.new_shells);
        self.stars.emit(self.new_stars.drain(..));
    }

    /// Nothing left burning or in flight (smoke may still hang).
    #[cfg(test)]
    pub fn is_dark(&self) -> bool {
        self.shells.is_empty() && self.stars.items.is_empty() && self.sparks.items.is_empty()
    }

    /// Burning stars, including comets and fuses riding on climbing shells.
    pub fn burning(&self) -> impl Iterator<Item = &Star> + Clone {
        self.stars.items.iter().chain(
            self.shells
                .iter()
                .flat_map(|shell| shell.attached.iter().flatten()),
        )
    }
}

/// Pools for testing one star or shell outside a world.
#[cfg(test)]
pub struct Sink {
    pub rng: Rng,
    pub stars: Vec<Star>,
    pub shells: Vec<Shell>,
    pub sparks: Particles<Spark>,
    pub puffs: Particles<Puff>,
}

#[cfg(test)]
impl Sink {
    pub fn new(sparks: usize) -> Self {
        Self {
            rng: Rng::new(42),
            stars: Vec::new(),
            shells: Vec::new(),
            sparks: Particles::new(sparks),
            puffs: Particles::new(1000),
        }
    }

    pub fn spawn(&mut self) -> Spawn<'_> {
        Spawn {
            rng: &mut self.rng,
            stars: &mut self.stars,
            shells: &mut self.shells,
            sparks: &mut self.sparks,
            puffs: &mut self.puffs,
            spark_rate: 1.0,
        }
    }
}
