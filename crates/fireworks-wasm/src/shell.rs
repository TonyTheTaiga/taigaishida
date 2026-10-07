//! Aerial shells: a paper casing, a bursting charge, and a payload of stars or
//! smaller shells packed at known radii. A time fuse lit by the lift charge
//! sets when the burst fires. Burst velocity comes from energy: the charge's
//! useful work, η·m·Q, becomes the payload's kinetic energy, with each petal's
//! speed proportional to its packing radius (self-similar expansion).

use crate::chemistry::{remaining_mass, Layer, StarRecipe, BLACK_POWDER_HEAT, TIME_FUSE};
use crate::particle::{Body, Clock, Particles, Vec3};
use crate::star::{Emission, Puff, Star};
use crate::{pick, rand, random_unit};

/// Fraction of the bursting charge's heat that becomes star motion. Shimizu
/// (Table 19) measured 63.5 m/s stars from a 6-inch perchlorate-burst shell;
/// the standard 6-inch load of 200 × 15 mm stars and 270 g of charge (Table
/// 23) gives η = ½·0.548 kg·63.5² / (0.27 kg · 2.89 MJ/kg) ≈ 0.0014. The same
/// table's chlorate and black-powder shells (71.4 and 56.7 m/s) differ in v²
/// by their heats of explosion, as this energy balance predicts.
pub const BURST_EFFICIENCY: f64 = 0.0014;

/// Fraction of the black-powder lift charge's heat that becomes shell motion.
/// Calibrated once: Shimizu's 120 g lift for a 7-inch shell (Table 27) must
/// carry the 2.76 kg Yae-zaki to the Japan Fireworks Association's 250 m,
/// which takes 113 m/s. Every other burst height then follows from each
/// shell's own mass, lift, and drag, and the muzzle speeds land near
/// Kosanke's "about 250 mph regardless of size".
pub const LIFT_EFFICIENCY: f64 = 0.0882;

pub enum Pattern {
    /// Stars evenly distributed over a sphere (chrysanthemum, peony).
    Sphere,
    /// Stars packed around the shell's equator (rings, Saturn).
    Ring,
}

pub enum Payload {
    Stars {
        recipe: StarRecipe,
        count: usize,
        pattern: Pattern,
        /// Packing radius as a fraction of the shell: 1 is the outer petal.
        position: f64,
    },
    Shells {
        designs: &'static [&'static ShellDesign],
        count: usize,
        position: f64,
        fuse: (f64, f64),
    },
}

pub struct ShellDesign {
    #[cfg_attr(not(test), allow(dead_code))]
    pub name: &'static str,
    pub diameter: f64,
    /// Paper casing plus the inert cores the bursting charge is pasted on.
    pub inert_mass: f64,
    pub burst_charge: f64,
    /// Heat of explosion of the bursting charge, J/kg.
    pub burst_heat: f64,
    /// Black-powder lift charge loaded under the shell in its mortar, kg.
    pub lift_charge: f64,
    pub payload: &'static [Payload],
    /// Optional comet glued to the casing: it rises with the shell and flies
    /// on after the burst (a palm's trunk).
    pub comet: StarRecipe,
}

/// The burning end of the time fuse, seen as a faint spark trail on the way
/// up. The burst consumes it.
const FUSE: StarRecipe = &[Layer {
    composition: &TIME_FUSE,
    thickness: 0.0045,
}];

impl ShellDesign {
    /// Mass and packing radius of each payload item.
    fn items(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        self.payload.iter().map(|item| match item {
            Payload::Stars {
                recipe,
                count,
                position,
                ..
            } => (*count as f64 * remaining_mass(recipe, 1.0, 0.0), *position),
            Payload::Shells {
                designs,
                count,
                position,
                ..
            } => (
                *count as f64 * designs.iter().map(|d| d.mass()).sum::<f64>()
                    / designs.len().max(1) as f64,
                *position,
            ),
        })
    }

    pub fn payload_mass(&self) -> f64 {
        self.items().map(|(mass, _)| mass).sum()
    }

    pub fn mass(&self) -> f64 {
        self.inert_mass + self.burst_charge + self.payload_mass()
    }

    /// Muzzle speed from the lift charge: ½·M·v² = η·m_lift·Q.
    pub fn muzzle_speed(&self) -> f64 {
        (2.0 * LIFT_EFFICIENCY * self.lift_charge * BLACK_POWDER_HEAT / self.mass()).sqrt()
    }

    /// Nominal flight from a mortar tilted `tilt` radians from vertical:
    /// where the shell tops out relative to the mortar, and when.
    pub fn flight(&self, tilt: f64) -> (Vec3, f64) {
        let speed = self.muzzle_speed();
        let mut body = Body::new(
            Vec3::default(),
            Vec3::new(speed * tilt.sin(), speed * tilt.cos(), 0.0),
            30.0,
        );
        body.mass = self.mass();
        body.diameter = self.diameter;
        let mut time = 0.0;
        while time < 30.0 && body.velocity.y > 0.0 {
            body.step(Clock::STEP_SECONDS);
            time += Clock::STEP_SECONDS;
        }
        (body.position, time)
    }

    /// Speed of the outer petal: ½·Σ mᵢ(v·rᵢ)² = η·m_burst·Q.
    pub fn burst_speed(&self) -> f64 {
        let energy = BURST_EFFICIENCY * self.burst_charge * self.burst_heat;
        let inertia: f64 = self.items().map(|(mass, r)| mass * r * r).sum();
        (2.0 * energy / inertia.max(1e-9)).sqrt()
    }
}

pub struct Shell {
    pub body: Body,
    pub design: &'static ShellDesign,
    pub fuse: f64,
    /// Things burning on the outside of the casing: the fuse, then any comet.
    pub attached: [Option<Star>; 2],
}

/// What a shell produces when its burst charge fires.
pub struct Burst<'a> {
    pub stars: &'a mut Vec<Star>,
    pub shells: &'a mut Vec<Shell>,
    pub puffs: &'a mut Particles<Puff>,
}

pub enum Flight {
    Climbing,
    Burst,
    /// The shell reached the water before its fuse burnt down.
    Lost,
}

impl Shell {
    pub fn new(design: &'static ShellDesign, position: Vec3, velocity: Vec3, fuse: f64) -> Self {
        let mut body = Body::new(position, velocity, 1e9);
        body.mass = design.mass();
        body.diameter = design.diameter;
        Self {
            body,
            design,
            fuse,
            attached: [
                Some(Star::new(FUSE, 1.0, position, velocity)),
                (!design.comet.is_empty())
                    .then(|| Star::new(design.comet, 1.0, position, velocity)),
            ],
        }
    }

    pub fn update(&mut self, dt: f64, out: &mut Emission) -> Flight {
        for slot in &mut self.attached {
            if let Some(star) = slot {
                star.body.position = self.body.position;
                star.body.velocity = self.body.velocity;
                if !star.update(dt, out) {
                    *slot = None;
                }
            }
        }
        self.body.step(dt);
        self.fuse -= dt;
        if self.fuse <= 0.0 {
            Flight::Burst
        } else if self.body.position.y <= 0.0 {
            Flight::Lost
        } else {
            Flight::Climbing
        }
    }

    pub fn burst(&mut self, out: &mut Burst) {
        let design = self.design;
        let origin = self.body.position;
        let speed = design.burst_speed();
        // Orient the shell: tilt its axis away from the line of sight so rings
        // read as ellipses of varying eccentricity.
        let tilt = rand(0.2, 1.15);
        let azimuth = rand(0.0, std::f64::consts::TAU);
        let pole = Vec3::new(
            tilt.sin() * azimuth.cos(),
            tilt.sin() * azimuth.sin(),
            tilt.cos(),
        );
        let (u, v) = pole.basis();
        let spin = rand(0.0, std::f64::consts::TAU);
        let golden = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());

        for item in design.payload {
            let (count, position) = match item {
                Payload::Stars {
                    count, position, ..
                }
                | Payload::Shells {
                    count, position, ..
                } => (*count, *position),
            };
            for i in 0..count {
                let direction = match item {
                    Payload::Stars {
                        pattern: Pattern::Ring,
                        ..
                    } => {
                        let a = spin + i as f64 / count as f64 * std::f64::consts::TAU;
                        u.scale(a.cos()).add(v.scale(a.sin()))
                    }
                    _ => {
                        let z = 1.0 - 2.0 * (i as f64 + 0.5) / count as f64;
                        let r = (1.0 - z * z).max(0.0).sqrt();
                        let a = spin + i as f64 * golden;
                        u.scale(r * a.cos())
                            .add(v.scale(r * a.sin()))
                            .add(pole.scale(z))
                    }
                }
                .add(random_unit().scale(0.03))
                .normalized();
                let start = origin.add(direction.scale(design.diameter * 0.5 * position));
                let velocity = self
                    .body
                    .velocity
                    .add(direction.scale(speed * position * rand(0.96, 1.04)));
                match item {
                    Payload::Stars { recipe, .. } => {
                        out.stars.push(Star::new(recipe, 1.0, start, velocity))
                    }
                    Payload::Shells { designs, fuse, .. } => out.shells.push(Shell::new(
                        pick(designs),
                        start,
                        velocity,
                        rand(fuse.0, fuse.1),
                    )),
                }
            }
        }

        if let Some(comet) = self.attached[1].take() {
            out.stars.push(comet);
        }
        // Flash and smoke scale with the charge, relative to a 6-inch shell.
        let size = (design.burst_charge / 0.27).cbrt();
        out.puffs.push(Puff::flash(origin, size.min(1.0)));
        let smoke = (4.0 + 10.0 * size).round() as usize;
        for _ in 0..smoke {
            let offset = random_unit().scale(design.diameter * rand(0.5, 6.0) * size);
            out.puffs.push(Puff::smoke(
                origin.add(offset),
                self.body.velocity.scale(0.2),
                rand(1.5, 3.0),
            ));
        }
    }
}
