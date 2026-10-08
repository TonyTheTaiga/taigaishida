//! Aerial shells: a paper casing, a bursting charge, and a payload of stars or
//! smaller shells packed at known radii. A time fuse lit by the lift charge
//! sets when the burst fires. Burst velocity comes from energy: the charge's
//! useful work, η·m·Q, becomes the payload's kinetic energy, with each petal's
//! speed proportional to its packing radius (self-similar expansion).

use crate::chemistry::{remaining_mass, Layer, StarRecipe, BLACK_POWDER_HEAT, TIME_FUSE};
use crate::particle::{Body, Clock, Particles, Vec3};
use crate::star::{Emission, Puff, Star};
use crate::{gauss, pick, rand, rand_f64, random_unit};

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
    /// One of `of` equal wedges around the shell's axis. Each wedge's stars
    /// carry a different dark delay, so a jikansa-botan lights quarter by
    /// quarter around the clock.
    Sector { index: usize, of: usize },
    /// One of `of` parallel slices across the shell; lit in turn, a
    /// slide-botan's face seems to slide across the sky.
    Slice { index: usize, of: usize },
    /// A flat picture (katamono): stars laid out on a unit template in the
    /// shell's equatorial plane, each thrown at a speed proportional to its
    /// distance from the centre so the picture grows without distorting.
    Template(fn(f64) -> (f64, f64)),
    /// The upper half of a sphere: a water shell bursting on the surface.
    Dome,
}

/// How a shell sits when it bursts.
#[derive(Clone, Copy, PartialEq)]
pub enum Orientation {
    /// Tumbled in flight: rings and pictures read at any angle.
    Tumbling,
    /// Loaded upright in the mortar and stabilised by its own spin, as makers
    /// do for pattern shells: the face mostly turns toward the audience.
    Upright,
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
    pub orientation: Orientation,
    /// A water shell: lobbed from the barge, it floats and bursts on the
    /// surface instead of on its time fuse.
    pub water: bool,
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

    /// A water shell's lob from a mortar tilted `tilt` radians: where it
    /// lands relative to the mortar, and when.
    pub fn splashdown(&self, tilt: f64) -> (Vec3, f64) {
        let speed = self.muzzle_speed();
        let mut body = Body::new(
            Vec3::new(0.0, crate::fleet::FREEBOARD, 0.0),
            Vec3::new(speed * tilt.sin(), speed * tilt.cos(), 0.0),
            30.0,
        );
        body.mass = self.mass();
        body.diameter = self.diameter;
        let mut time = 0.0;
        while time < 30.0 && !(body.position.y <= 0.3 && body.velocity.y < 0.0) {
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
        if self.design.water && self.body.position.y <= 0.3 && self.body.velocity.y < 0.0 {
            self.body.position.y = 0.3;
            self.body.velocity = Vec3::default();
            Flight::Burst
        } else if self.fuse <= 0.0 && !self.design.water {
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
        // Orient the shell. A tumbled shell's axis tilts away from the line
        // of sight so rings read as ellipses of varying eccentricity; an
        // upright one faces the audience within a few tens of degrees.
        let (tilt, azimuth) = match design.orientation {
            Orientation::Tumbling => (rand(0.2, 1.15), rand(0.0, std::f64::consts::TAU)),
            Orientation::Upright => (rand(0.0, 0.45), rand(0.0, std::f64::consts::TAU)),
        };
        let pole = Vec3::new(
            tilt.sin() * azimuth.cos(),
            tilt.sin() * azimuth.sin(),
            -tilt.cos(),
        );
        let (u, v, spin) = match design.orientation {
            Orientation::Tumbling => {
                let (u, v) = pole.basis();
                (u, v, rand(0.0, std::f64::consts::TAU))
            }
            // Picture axes: across and up as the audience sees them, rolled
            // by up to 30°.
            Orientation::Upright => {
                let across = Vec3::new(0.0, 1.0, 0.0)
                    .cross(pole)
                    .normalized()
                    .scale(-1.0);
                let up = pole.cross(across).scale(-1.0);
                (across, up, rand(-0.5, 0.5))
            }
        };
        let (u, v) = (
            u.scale(spin.cos()).add(v.scale(spin.sin())),
            v.scale(spin.cos()).sub(u.scale(spin.sin())),
        );
        let golden = std::f64::consts::PI * (3.0 - 5.0_f64.sqrt());
        let fraction = |i: usize| (i as f64 * 0.618_033_988_75).fract();
        // Hand-packed shells never open perfectly (estimates): the charge's
        // strength varies by about 6%, the casing tears unevenly so one side
        // flies up to 12% faster, and the flower comes out a little flattened
        // or drawn out along a random axis.
        let strength = (1.0 + 0.06 * gauss()).clamp(0.85, 1.15);
        let tear = random_unit();
        let lopsided = rand(0.0, 0.12);
        let squash = random_unit();
        let flattening = rand(-0.1, 0.1);

        for item in design.payload {
            let (count, position) = match item {
                Payload::Stars {
                    count, position, ..
                }
                | Payload::Shells {
                    count, position, ..
                } => (*count, *position),
            };
            let pattern = match item {
                Payload::Stars { pattern, .. } => pattern,
                Payload::Shells { .. } => &Pattern::Sphere,
            };
            for i in 0..count {
                let sphere = |i: usize| {
                    let z = 1.0 - 2.0 * (i as f64 + 0.5) / count as f64;
                    let r = (1.0 - z * z).max(0.0).sqrt();
                    let a = i as f64 * golden;
                    u.scale(r * a.cos())
                        .add(v.scale(r * a.sin()))
                        .add(pole.scale(z))
                };
                let jitter = random_unit().scale(0.05);
                let offset = match *pattern {
                    Pattern::Sphere => sphere(i).add(jitter).normalized(),
                    Pattern::Ring => {
                        let a = i as f64 / count as f64 * std::f64::consts::TAU;
                        u.scale(a.cos())
                            .add(v.scale(a.sin()))
                            .add(jitter)
                            .normalized()
                    }
                    Pattern::Sector { index, of } => {
                        let z = 1.0 - 2.0 * (i as f64 + 0.5) / count as f64;
                        let r = (1.0 - z * z).max(0.0).sqrt();
                        let a = (index as f64 + fraction(i)) / of as f64 * std::f64::consts::TAU;
                        u.scale(r * a.cos())
                            .add(v.scale(r * a.sin()))
                            .add(pole.scale(z))
                            .add(jitter)
                            .normalized()
                    }
                    Pattern::Slice { index, of } => {
                        // Equal widths along an axis cut equal areas of a sphere.
                        let across = -1.0 + 2.0 * (index as f64 + fraction(i)) / of as f64;
                        let r = (1.0 - across * across).max(0.0).sqrt();
                        let a = i as f64 * golden;
                        u.scale(across)
                            .add(v.scale(r * a.cos()))
                            .add(pole.scale(r * a.sin()))
                            .add(jitter)
                            .normalized()
                    }
                    Pattern::Template(shape) => {
                        let (x, y) = shape((i as f64 + 0.5) / count as f64);
                        u.scale(x).add(v.scale(y)).add(jitter.scale(0.4))
                    }
                    Pattern::Dome => {
                        let up = Vec3::new(0.0, 1.0, 0.0);
                        let (a, b) = up.basis();
                        let y = 1.0 - (i as f64 + 0.5) / count as f64;
                        let r = (1.0 - y * y).max(0.0).sqrt();
                        let turn = i as f64 * golden;
                        a.scale(r * turn.cos())
                            .add(b.scale(r * turn.sin()))
                            .add(up.scale(y))
                            .add(jitter)
                            .normalized()
                    }
                };
                let direction = offset.normalized();
                // About one star in fifty never takes fire and falls dark.
                if rand_f64() < 0.02 {
                    continue;
                }
                let along = direction.dot(squash);
                let shape = strength
                    * (1.0 + lopsided * direction.dot(tear))
                    * (1.0 + flattening * (1.5 * along * along - 0.5))
                    * (1.0 + 0.05 * gauss()).clamp(0.85, 1.15);
                let start = origin.add(offset.scale(design.diameter * 0.5 * position));
                let velocity = self
                    .body
                    .velocity
                    .add(offset.scale(speed * position * shape));
                match item {
                    Payload::Stars { recipe, .. } => out
                        .stars
                        .push(Star::new(recipe, 1.0, start, velocity).varied().primed(0.1)),
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
        // Flash and smoke scale with the charge, relative to a 6-inch shell:
        // the bursting charge leaves a dense cloud at the centre, and the
        // stars leave theirs where they burn out.
        let size = (design.burst_charge / 0.27).cbrt();
        out.puffs.push(Puff::flash(origin, size.min(1.0)));
        let smoke = (2.0 + 3.0 * size).round() as usize;
        for _ in 0..smoke {
            let offset = random_unit().scale(rand(1.0, 6.0) * size);
            out.puffs.push(Puff::smoke(
                origin.add(offset),
                self.body.velocity.scale(0.2),
                rand(4.0, 7.0) * size.max(0.3),
                0.35,
                rand(18.0, 26.0),
            ));
        }
    }
}
