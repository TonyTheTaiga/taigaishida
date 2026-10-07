//! Burning bodies built from compositions.
//!
//! A `Star` is a layered sphere. Its flame front regresses inward at the
//! current layer's burn rate, so mass, diameter, drag, light, spark output, and
//! thrust all change continuously; crossing a layer boundary changes colour or
//! triggers that layer's effect. A `Spark` is an incandescent particle whose
//! light comes only from its temperature: it smoulders, burns, then cools by
//! convection and radiation until it is no longer visible.

use crate::chemistry::{
    incandescence, recipe_radius, remaining_mass, Effect, Layer, Rgb, SparkFuel, BLACK_POWDER_HEAT,
};
use crate::particle::{Body, Particles, Vec3, GRAVITY};
use crate::trail::Trail;
use crate::{rand, random_unit};

/// A 4 mm star of unit luminosity has a luminance of 1.
pub const REFERENCE_RADIUS: f64 = 0.004;
/// Sparks below this temperature emit too little visible light to matter.
const VISIBLE_TEMPERATURE: f64 = 900.0;
const AMBIENT_TEMPERATURE: f64 = 290.0;
const STEFAN_BOLTZMANN: f64 = 5.670_374e-8;
const SPARK_EMISSIVITY: f64 = 0.9;
/// Thrust is limited as a star's last fraction of a millimetre burns away.
const MAX_THRUST_ACCELERATION: f64 = 140.0;
const UNBOUNDED_LIFE: f64 = 1e9;

/// Where a burning star sends the particles it creates this tick.
pub struct Emission<'a> {
    pub sparks: &'a mut Particles<Spark>,
    pub stars: &'a mut Vec<Star>,
    /// 0..=1 sampling rate that keeps the spark budget from saturating.
    pub spark_rate: f64,
}

pub struct Star {
    pub body: Body,
    pub layers: &'static [Layer],
    /// Geometric scale applied to every layer (fragments are smaller copies).
    pub scale: f64,
    /// Depth burned from the outer surface, metres.
    pub burned: f64,
    pub age: f64,
    pub trail: Trail,
    phase: f64,
    heading: Vec3,
    spark_debt: f64,
}

impl Star {
    pub fn new(layers: &'static [Layer], scale: f64, position: Vec3, velocity: Vec3) -> Self {
        let mut star = Self {
            body: Body::new(position, velocity, UNBOUNDED_LIFE),
            layers,
            scale,
            burned: 0.0,
            age: 0.0,
            trail: Trail::new(),
            phase: rand(0.0, 1.0),
            heading: random_unit(),
            spark_debt: rand(0.0, 1.0),
        };
        star.sync_body();
        star
    }

    pub fn radius(&self) -> f64 {
        (recipe_radius(self.layers) * self.scale - self.burned).max(0.0)
    }

    /// Index of the burning layer and the burn depth at which it ends.
    fn current_layer(&self) -> Option<(usize, f64)> {
        let mut depth = 0.0;
        for (index, layer) in self.layers.iter().enumerate() {
            depth += layer.thickness * self.scale;
            if self.burned < depth {
                return Some((index, depth));
            }
        }
        None
    }

    pub fn layer(&self) -> Option<&'static Layer> {
        let layers = self.layers;
        self.current_layer().map(|(index, _)| &layers[index])
    }

    fn sync_body(&mut self) {
        self.body.mass = remaining_mass(self.layers, self.scale, self.burned);
        self.body.diameter = 2.0 * self.radius();
    }

    /// Advance combustion and motion. Returns false once the star is spent,
    /// has split, or has fallen into the water.
    pub fn update(&mut self, dt: f64, out: &mut Emission) -> bool {
        let Some((index, layer_end)) = self.current_layer() else {
            return false;
        };
        let composition = self.layers[index].composition;
        if let Effect::Split {
            fragments,
            efficiency,
        } = composition.effect
        {
            self.split(index, fragments, efficiency, out);
            return false;
        }
        self.trail.record(self.body.position, 1);

        let mass_before = self.body.mass;
        self.burned = (self.burned + composition.burn_rate * dt).min(layer_end);
        self.sync_body();
        let burned_mass = (mass_before - self.body.mass).max(0.0);

        if let Some(fuel) = &composition.sparks {
            self.spark_debt += burned_mass * 1000.0 * fuel.per_gram * out.spark_rate;
            while self.spark_debt >= 1.0 {
                self.spark_debt -= 1.0;
                // Spread emission along this tick's path so tails stay continuous.
                let along = self.body.velocity.scale(dt * rand(0.0, 1.0));
                let throw = random_unit().scale(fuel.eject_speed * rand(0.3, 1.0));
                out.sparks.push(Spark::new(
                    fuel,
                    self.body.position.add(along),
                    self.body.velocity.add(throw),
                ));
            }
        }

        self.body.acceleration = match composition.effect {
            Effect::Thrust {
                exhaust_speed,
                wander,
            } if self.body.mass > 0.0 => {
                self.heading = self
                    .heading
                    .add(random_unit().scale(wander * dt.sqrt()))
                    .normalized();
                let thrust = burned_mass / dt * exhaust_speed;
                self.heading
                    .scale((thrust / self.body.mass).min(MAX_THRUST_ACCELERATION))
            }
            _ => Vec3::default(),
        };

        self.body.step(dt);
        self.age += dt;
        self.body.position.y > 0.0 && self.radius() > 0.0
    }

    /// The split charge fires: the remaining core breaks into equal fragments
    /// thrown perpendicular to the flight path. Momentum is conserved and the
    /// fragments share the charge's useful energy as kinetic energy.
    fn split(&self, index: usize, fragments: usize, efficiency: f64, out: &mut Emission) {
        let core = &self.layers[index + 1..];
        if core.is_empty() || fragments == 0 {
            return;
        }
        let core_mass = remaining_mass(core, self.scale, 0.0);
        let charge = remaining_mass(&self.layers[index..], self.scale, 0.0) - core_mass;
        let energy = charge * BLACK_POWDER_HEAT * efficiency;
        let speed = (2.0 * energy / core_mass).sqrt();
        let fragment_scale = self.scale * (fragments as f64).powf(-1.0 / 3.0);
        let (u, v) = self.body.velocity.basis();
        let roll = rand(0.0, std::f64::consts::TAU);
        for k in 0..fragments {
            let angle = roll + k as f64 / fragments as f64 * std::f64::consts::TAU;
            let direction = u.scale(angle.cos()).add(v.scale(angle.sin()));
            out.stars.push(Star::new(
                core,
                fragment_scale,
                self.body.position,
                self.body.velocity.add(direction.scale(speed)),
            ));
        }
    }

    /// Emitter colour and luminance relative to the reference star. Light is
    /// proportional to burning surface area; strobes concentrate the same
    /// energy into short pulses.
    pub fn light(&self) -> Option<(Rgb, f64)> {
        let composition = self.layer()?.composition;
        let radius = self.radius() / REFERENCE_RADIUS;
        let mut luminance = composition.luminosity * radius * radius;
        if let Effect::Strobe { hz, duty } = composition.effect {
            let cycle = (self.age * hz + self.phase).fract();
            luminance = if cycle < duty { luminance / duty } else { 0.0 };
        }
        Some((composition.color, luminance))
    }
}

pub struct Spark {
    pub body: Body,
    pub temperature: f64,
    heat_capacity: f64,
    delay: f64,
    smoulder_temperature: f64,
    burn: f64,
    burn_temperature: f64,
}

impl Spark {
    pub fn new(fuel: &SparkFuel, position: Vec3, velocity: Vec3) -> Self {
        let mut body = Body::new(position, velocity, UNBOUNDED_LIFE);
        body.diameter = fuel.diameter;
        body.mass = fuel.density * std::f64::consts::PI / 6.0 * fuel.diameter.powi(3);
        let delay = rand(fuel.delay.0, fuel.delay.1);
        Self {
            body,
            temperature: if delay > 0.0 {
                fuel.smoulder_temperature
            } else {
                fuel.burn_temperature
            },
            heat_capacity: fuel.heat_capacity,
            delay,
            smoulder_temperature: fuel.smoulder_temperature,
            burn: rand(fuel.burn_time.0, fuel.burn_time.1),
            burn_temperature: fuel.burn_temperature,
        }
    }

    pub fn update(&mut self, dt: f64) -> bool {
        if self.delay > 0.0 {
            self.delay -= dt;
            self.temperature = self.smoulder_temperature;
        } else if self.burn > 0.0 {
            self.burn -= dt;
            // Combustion at the particle surface flickers by a few percent.
            self.temperature = self.burn_temperature * rand(0.97, 1.03);
        } else {
            self.cool(dt);
        }
        self.body.step(dt);
        (self.delay > 0.0 || self.temperature > VISIBLE_TEMPERATURE) && self.body.position.y > 0.0
    }

    /// Lumped-capacitance cooling: conduction to air (Nu = 2) plus grey-body
    /// radiation, dT/dt = -6 / (ρ c d) · [h (T - Tₐ) + εσ (T⁴ - Tₐ⁴)].
    /// Air conductivity is taken at the film temperature from a linear fit to
    /// tabulated values (0.026 W/(m·K) at 300 K, 0.067 at 1000 K, 0.096 at 1500 K).
    fn cool(&mut self, dt: f64) {
        let d = self.body.diameter;
        let density = self.body.mass / (std::f64::consts::PI / 6.0 * d.powi(3));
        let t = self.temperature;
        let film = 0.5 * (t + AMBIENT_TEMPERATURE);
        let conductivity = 0.0263 + 5.8e-5 * (film - 300.0);
        let h = 2.0 * conductivity / d;
        let flux = h * (t - AMBIENT_TEMPERATURE)
            + SPARK_EMISSIVITY * STEFAN_BOLTZMANN * (t.powi(4) - AMBIENT_TEMPERATURE.powi(4));
        let rate = 6.0 / (density * self.heat_capacity * d) * flux;
        self.temperature = (t - rate * dt).max(AMBIENT_TEMPERATURE);
    }

    /// Visible luminance relative to a 0.3 mm particle at 2200 K.
    pub fn luminance(&self) -> f64 {
        let size = self.body.diameter / 0.0003;
        incandescence(self.temperature) * size * size
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum PuffKind {
    Smoke,
    Flash,
}

/// Burst by-products: a short detonation flash and drifting combustion smoke.
pub struct Puff {
    pub body: Body,
    pub kind: PuffKind,
    pub intensity: f64,
}

impl Puff {
    pub fn smoke(position: Vec3, velocity: Vec3, life: f64) -> Self {
        let mut body = Body::new(position, velocity, life);
        // A parcel of hot gas and K₂CO₃/K₂SO₄ particulate: it rides the wind
        // and rises slowly on its own buoyancy.
        body.diameter = 0.05;
        body.mass = 1e-5;
        body.acceleration.y = GRAVITY + 0.3;
        Self {
            body,
            kind: PuffKind::Smoke,
            intensity: 1.0,
        }
    }

    pub fn flash(position: Vec3, intensity: f64) -> Self {
        let mut body = Body::new(position, Vec3::default(), 0.17);
        body.diameter = 0.0;
        body.acceleration.y = GRAVITY;
        Self {
            body,
            kind: PuffKind::Flash,
            intensity,
        }
    }

    pub fn fraction(&self) -> f64 {
        (self.body.life / self.body.max_life).max(0.0)
    }

    pub fn update(&mut self, dt: f64) -> bool {
        self.body.step(dt);
        self.body.life > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chemistry::*;

    const COLOUR_CHANGER: &[Layer] = &[
        Layer {
            composition: &STRONTIUM_RED,
            thickness: 0.002,
        },
        Layer {
            composition: &BARIUM_GREEN,
            thickness: 0.002,
        },
    ];

    const CROSSETTE: &[Layer] = &[
        Layer {
            composition: &TITANIUM_SILVER,
            thickness: 0.0005,
        },
        Layer {
            composition: &SPLIT_CHARGE,
            thickness: 0.0004,
        },
        Layer {
            composition: &TITANIUM_SILVER,
            thickness: 0.002,
        },
    ];

    fn run(star: &mut Star, sparks: &mut Particles<Spark>, stars: &mut Vec<Star>) -> bool {
        star.update(
            1.0 / 60.0,
            &mut Emission {
                sparks,
                stars,
                spark_rate: 1.0,
            },
        )
    }

    #[test]
    fn stars_shrink_lose_mass_and_change_colour_as_layers_burn() {
        let mut sparks = Particles::new(0);
        let mut stars = Vec::new();
        let mut star = Star::new(
            COLOUR_CHANGER,
            1.0,
            Vec3::new(0.0, 100.0, 0.0),
            Vec3::new(30.0, 0.0, 0.0),
        );
        let start = star.light().unwrap();
        assert_eq!(start.0, STRONTIUM_RED.color);
        let mass = star.body.mass;
        let mut ticks = 0;
        while star.layer().map(|l| l.composition.color) == Some(STRONTIUM_RED.color) {
            assert!(run(&mut star, &mut sparks, &mut stars));
            ticks += 1;
        }
        // 2 mm at the red star's measured regression rate.
        let expected = 0.002 / STRONTIUM_RED.burn_rate * 60.0;
        assert!(
            (ticks as f64 - expected).abs() <= 1.0,
            "{ticks} vs {expected}"
        );
        assert_eq!(star.light().unwrap().0, BARIUM_GREEN.color);
        assert!(star.body.mass < mass * 0.2);
        assert!(star.light().unwrap().1 < start.1);
        while run(&mut star, &mut sparks, &mut stars) {}
        assert_eq!(star.radius(), 0.0);
        assert!(star.light().is_none());
    }

    #[test]
    fn crossette_split_conserves_mass_and_momentum() {
        let mut sparks = Particles::new(10_000);
        let mut stars = Vec::new();
        let velocity = Vec3::new(20.0, 10.0, -5.0);
        let mut star = Star::new(CROSSETTE, 1.0, Vec3::new(0.0, 100.0, 0.0), velocity);
        while run(&mut star, &mut sparks, &mut stars) {}
        assert!(!sparks.items.is_empty(), "titanium tail throws sparks");
        assert_eq!(stars.len(), 4);
        let core = remaining_mass(&CROSSETTE[2..], 1.0, 0.0);
        let total: f64 = stars.iter().map(|s| s.body.mass).sum();
        assert!((total - core).abs() < 1e-12);
        let mean = stars.iter().fold(Vec3::default(), |acc, s| {
            acc.add(s.body.velocity.scale(0.25))
        });
        assert!(mean.sub(star.body.velocity).length() < 1e-9);
        for fragment in &stars {
            let kick = fragment.body.velocity.sub(star.body.velocity);
            assert!(
                kick.dot(star.body.velocity).abs() < 1e-6,
                "split is sideways"
            );
            assert!((15.0..60.0).contains(&kick.length()));
        }
    }

    #[test]
    fn sparks_cool_out_after_burning_and_glitter_waits_to_flash() {
        let mut ember = Spark::new(&CHARCOAL_SPARKS, Vec3::new(0.0, 50.0, 0.0), Vec3::default());
        let mut alive_ticks = 0;
        while ember.update(1.0 / 60.0) {
            alive_ticks += 1;
            assert!(alive_ticks < 120);
        }
        assert!(ember.temperature < VISIBLE_TEMPERATURE);
        assert!(ember.luminance() < 1e-4);

        let glitter = GLITTER_GOLD.sparks.as_ref().unwrap();
        let mut flake = Spark::new(glitter, Vec3::new(0.0, 50.0, 0.0), Vec3::default());
        let smoulder = flake.luminance();
        let mut peak: f64 = 0.0;
        while flake.update(1.0 / 60.0) {
            peak = peak.max(flake.luminance());
        }
        assert!(peak > smoulder * 100.0);
    }

    #[test]
    fn strobes_pulse_and_fish_swim_under_thrust() {
        const STROBE: &[Layer] = &[Layer {
            composition: &TWINKLER_GREEN,
            thickness: 0.003,
        }];
        const FISH: &[Layer] = &[Layer {
            composition: &FISH_FUEL,
            thickness: 0.003,
        }];
        let mut sparks = Particles::new(10_000);
        let mut stars = Vec::new();
        let mut strobe = Star::new(STROBE, 1.0, Vec3::new(0.0, 100.0, 0.0), Vec3::default());
        let mut lit = 0;
        let mut dark = 0;
        for _ in 0..60 {
            run(&mut strobe, &mut sparks, &mut stars);
            if strobe.light().unwrap().1 > 0.0 {
                lit += 1;
            } else {
                dark += 1;
            }
        }
        assert!(lit > 5 && dark > lit);

        let mut fish = Star::new(FISH, 1.0, Vec3::new(0.0, 100.0, 0.0), Vec3::default());
        run(&mut fish, &mut sparks, &mut stars);
        assert!(fish.body.acceleration.length() > 20.0);
        for _ in 0..30 {
            run(&mut fish, &mut sparks, &mut stars);
        }
        assert!(fish.body.velocity.length() > 5.0);
    }
}
