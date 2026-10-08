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
use crate::particle::{Body, Particles, Vec3, DISPLAY_WIND, GRAVITY};
use crate::trail::Trail;
use crate::{gauss, rand, random_unit};

/// A 4 mm star of unit luminosity has a luminance of 1.
pub const REFERENCE_RADIUS: f64 = 0.004;
/// Sparks below this temperature emit too little visible light to matter.
const VISIBLE_TEMPERATURE: f64 = 900.0;
const AMBIENT_TEMPERATURE: f64 = 290.0;
const STEFAN_BOLTZMANN: f64 = 5.670_374e-8;
const SPARK_EMISSIVITY: f64 = 0.9;
/// Thrust is limited as a star's last fraction of a millimetre burns away.
const MAX_THRUST_ACCELERATION: f64 = 140.0;

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
    /// Burn-rate multiplier: hand-pressed stars differ in density and mix.
    vigour: f64,
    /// Brightness multiplier from the same differences and from impurities.
    glow: f64,
    /// Seconds left before the priming catches and the star lights.
    delay: f64,
    mount: Mount,
}

/// How a burning star is held.
#[derive(Clone, Copy, PartialEq)]
enum Mount {
    /// Flying under gravity, drag, and its own thrust.
    Free,
    /// Clamped in a tube on deck, like a gerb.
    Fixed,
}

impl Star {
    pub fn new(layers: &'static [Layer], scale: f64, position: Vec3, velocity: Vec3) -> Self {
        let mut star = Self {
            body: Body::new(position, velocity),
            layers,
            scale,
            burned: 0.0,
            age: 0.0,
            trail: Trail::new(),
            phase: rand(0.0, 1.0),
            heading: random_unit(),
            spark_debt: rand(0.0, 1.0),
            vigour: 1.0,
            glow: 1.0,
            delay: 0.0,
            mount: Mount::Free,
        };
        star.sync_body();
        star
    }

    /// No two hand-made stars are alike: burn rate varies by about 6% and
    /// brightness by about 10% (estimates within the scatter of Ooki's
    /// measured burn rates), so a flower's stars die over a spread of time
    /// rather than in one frame.
    pub fn varied(mut self) -> Self {
        self.vigour = (1.0 + 0.06 * gauss()).clamp(0.82, 1.18);
        self.glow = (1.0 + 0.1 * gauss()).clamp(0.7, 1.3);
        self
    }

    /// Clamp the star in place: it burns where it stands, as a gerb does in
    /// its tube on deck.
    pub fn mounted(mut self) -> Self {
        self.mount = Mount::Fixed;
        self
    }

    /// The burst flame takes up to `latest` seconds to light the priming;
    /// until then the star flies dark.
    pub fn primed(mut self, latest: f64) -> Self {
        self.delay = rand(0.0, latest);
        self
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
        if self.delay > 0.0 {
            self.delay -= dt;
            self.trail.record(self.body.position, 1);
            self.body.step(dt);
            return self.body.position.y > 0.0;
        }
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
        self.burned = (self.burned + composition.burn_rate * self.vigour * dt).min(layer_end);
        self.sync_body();
        let burned_mass = (mass_before - self.body.mass).max(0.0);

        if let Some(fuel) = &composition.sparks {
            self.spark_debt += burned_mass * 1000.0 * fuel.per_gram * out.spark_rate;
            while self.spark_debt >= 1.0 {
                self.spark_debt -= 1.0;
                // Spread emission along this tick's path so tails stay continuous.
                let along = self.body.velocity.scale(dt * rand(0.0, 1.0));
                let throw = match composition.effect {
                    Effect::Fountain { spread } => Vec3::new(0.0, 1.0, 0.0)
                        .add(random_unit().scale(spread))
                        .normalized()
                        .scale(fuel.eject_speed * rand(0.7, 1.0)),
                    _ => random_unit().scale(fuel.eject_speed * rand(0.3, 1.0)),
                };
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
            Effect::Flutter { hz, glide } => {
                // Lift from the tumbling flake pushes it to and fro across
                // a fixed horizontal heading.
                let across = Vec3::new(self.heading.x, 0.0, self.heading.z).normalized();
                let swing = (std::f64::consts::TAU * hz * self.age + self.phase * 6.0).sin();
                across.scale(glide * GRAVITY * swing)
            }
            _ => Vec3::default(),
        };

        if self.mount == Mount::Free {
            self.body.step(dt);
        }
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
            out.stars.push(
                Star::new(
                    core,
                    fragment_scale,
                    self.body.position,
                    self.body.velocity.add(direction.scale(speed)),
                )
                .varied(),
            );
        }
    }

    /// Emitter colour and luminance relative to the reference star. Light is
    /// proportional to burning surface area; strobes concentrate the same
    /// energy into short pulses.
    pub fn light(&self) -> Option<(Rgb, f64)> {
        if self.delay > 0.0 {
            return None;
        }
        let composition = self.layer()?.composition;
        let radius = self.radius() / REFERENCE_RADIUS;
        // Flames flutter by a few percent as gas and particles leave the
        // surface unevenly.
        let flutter = 1.0
            + 0.07
                * (self.age * 41.0 + self.phase * 31.0).sin()
                * (self.age * 17.0 + self.phase * 11.0).sin();
        let mut luminance = composition.luminosity * radius * radius * self.glow * flutter;
        match composition.effect {
            Effect::Strobe { hz, duty } => {
                let cycle = (self.age * hz + self.phase).fract();
                luminance = if cycle < duty { luminance / duty } else { 0.0 };
            }
            // The flake's coated face turns toward and away from the viewer.
            Effect::Flutter { hz, .. } => {
                let face = (std::f64::consts::PI * hz * self.age + self.phase * 3.0).sin();
                luminance *= 0.25 + 1.5 * face * face;
            }
            _ => {}
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
        let mut body = Body::new(position, velocity);
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

/// Turbulent mixing spreads a smoke parcel's radius as √t; a burst cloud
/// doubles in size within ten seconds in light wind (estimate).
const SMOKE_SPREAD: f64 = 2.2;

/// A smoke parcel rises on its own heat at about 0.3 m/s (estimate).
const SMOKE_RISE: f64 = 0.3;
/// Seconds for a parcel thrown out by a burst or lift to settle into the wind.
const SMOKE_SETTLING: f64 = 2.0;

/// Burst by-products: a short detonation flash and drifting combustion smoke.
/// Neither is a falling body: a flash stays where its charge fired, and smoke
/// is carried by the air, so both move kinematically.
pub struct Puff {
    pub position: Vec3,
    velocity: Vec3,
    life: f64,
    max_life: f64,
    pub kind: PuffKind,
    /// Flash: light relative to a 6-inch burst. Smoke: optical depth at the
    /// parcel's centre when it forms.
    pub intensity: f64,
    /// Smoke radius when the parcel forms, metres.
    initial_radius: f64,
}

impl Puff {
    /// A parcel of hot gas and K₂CO₃/K₂SO₄ particulate. It settles into the
    /// wind, rises slowly on its own heat, and thins as it spreads; it hangs
    /// in the sky long enough for later bursts to light it.
    pub fn smoke(position: Vec3, velocity: Vec3, radius: f64, density: f64, life: f64) -> Self {
        Self {
            position,
            velocity,
            life,
            max_life: life,
            kind: PuffKind::Smoke,
            intensity: density,
            initial_radius: radius,
        }
    }

    /// The fireball of a burst or lift: a sixth of a second of light.
    pub fn flash(position: Vec3, intensity: f64) -> Self {
        Self {
            position,
            velocity: Vec3::default(),
            life: 0.17,
            max_life: 0.17,
            kind: PuffKind::Flash,
            intensity,
            initial_radius: 0.0,
        }
    }

    pub fn fraction(&self) -> f64 {
        (self.life / self.max_life).max(0.0)
    }

    pub fn age(&self) -> f64 {
        self.max_life - self.life
    }

    /// Current smoke radius, metres.
    pub fn radius(&self) -> f64 {
        self.initial_radius + SMOKE_SPREAD * self.age().sqrt()
    }

    /// Optical depth through the parcel's centre: the same particulate spread
    /// over a growing area, faded in over the first second and out at the end.
    pub fn density(&self) -> f64 {
        let spread = (self.initial_radius / self.radius()).powi(2);
        let form = (self.age() / 0.8).min(1.0);
        let fade = (self.fraction() / 0.3).min(1.0);
        self.intensity * spread * form * fade
    }

    pub fn update(&mut self, dt: f64) -> bool {
        if self.kind == PuffKind::Smoke {
            let drift = DISPLAY_WIND.add(Vec3::new(0.0, SMOKE_RISE, 0.0));
            let settle = (dt / SMOKE_SETTLING).min(1.0);
            self.velocity = self.velocity.add(drift.sub(self.velocity).scale(settle));
            self.position = self.position.add(self.velocity.scale(dt));
        }
        self.life -= dt;
        self.life > 0.0
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

    #[test]
    fn a_gerb_stays_in_its_tube_and_jets_upward() {
        const GERB: &[Layer] = &[Layer {
            composition: &GERB_SILVER,
            thickness: 0.02,
        }];
        let mut sparks = Particles::new(10_000);
        let mut stars = Vec::new();
        let deck = Vec3::new(5.0, 1.8, -2.0);
        let mut gerb = Star::new(GERB, 1.0, deck, Vec3::default()).mounted();
        for _ in 0..120 {
            assert!(run(&mut gerb, &mut sparks, &mut stars));
        }
        assert_eq!(gerb.body.position, deck);
        assert!(sparks.items.len() > 50);
        let rising = sparks
            .items
            .iter()
            .filter(|s| s.body.velocity.y > 0.0)
            .count();
        assert!(rising * 2 > sparks.items.len(), "the jet points up");
    }

    #[test]
    fn smoke_settles_into_the_wind_and_rises() {
        let mut smoke = Puff::smoke(
            Vec3::new(0.0, 200.0, 0.0),
            Vec3::new(-20.0, 0.0, 5.0),
            5.0,
            0.3,
            30.0,
        );
        // Ten settling times.
        for _ in 0..1200 {
            assert!(smoke.update(1.0 / 60.0));
        }
        let drift = DISPLAY_WIND.add(Vec3::new(0.0, SMOKE_RISE, 0.0));
        assert!(smoke.velocity.sub(drift).length() < 0.05);
        assert!(smoke.position.y > 200.0, "smoke never falls");
        assert!(smoke.radius() > 5.0 && smoke.density() < 0.3);
        let mut flash = Puff::flash(Vec3::new(1.0, 2.0, 3.0), 1.0);
        flash.update(0.1);
        assert_eq!(flash.position, Vec3::new(1.0, 2.0, 3.0));
        assert!(!flash.update(0.1));
    }
}
