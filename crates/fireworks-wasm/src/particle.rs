//! Compact 3D particle core. Positions use metres, velocities m/s, and time seconds.
//! World Y is altitude (up); the water surface is the XZ plane at Y=0.
//! Effects own emission and appearance; this module owns motion, time, and capacity.

pub const GRAVITY: f64 = 9.80665;
/// Sea-level air at 20 °C.
pub const AIR_DENSITY: f64 = 1.204;
/// Isothermal scale height of the lower atmosphere: air thins about 3% by the
/// 250 m burst height of a 7-inch shell.
const SCALE_HEIGHT: f64 = 8434.0;
const AIR_DYNAMIC_VISCOSITY: f64 = 1.81e-5;
const DEFAULT_STAR_MASS: f64 = 0.001;
const DEFAULT_STAR_DIAMETER: f64 = 0.010;
const DISPLAY_WIND: Vec3 = Vec3 {
    x: 2.0,
    y: 0.0,
    z: 0.8,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }

    pub fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }

    pub fn scale(self, k: f64) -> Vec3 {
        Vec3::new(self.x * k, self.y * k, self.z * k)
    }

    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    pub fn normalized(self) -> Vec3 {
        let length = self.length();
        if length > 1e-12 {
            self.scale(1.0 / length)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        }
    }

    /// Two unit vectors that complete a right-handed basis with `self`.
    pub fn basis(self) -> (Vec3, Vec3) {
        let n = self.normalized();
        let helper = if n.x.abs() < 0.9 {
            Vec3::new(1.0, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 1.0, 0.0)
        };
        let u = helper.cross(n).normalized();
        (u, n.cross(u))
    }
}

pub struct Body {
    pub position: Vec3,
    pub velocity: Vec3,
    pub acceleration: Vec3,
    pub wind: Vec3,
    pub mass: f64,
    pub diameter: f64,
    pub life: f64,
    pub max_life: f64,
}

impl Body {
    pub fn new(position: Vec3, velocity: Vec3, life: f64) -> Self {
        Self {
            position,
            velocity,
            acceleration: Vec3::default(),
            wind: DISPLAY_WIND,
            mass: DEFAULT_STAR_MASS,
            diameter: DEFAULT_STAR_DIAMETER,
            life,
            max_life: life,
        }
    }

    fn drag_coefficient(reynolds: f64) -> f64 {
        if reynolds < 1e-8 {
            0.0
        } else if reynolds < 1000.0 {
            24.0 / reynolds * (1.0 + 0.15 * reynolds.powf(0.687))
        } else {
            // Ooki et al. (2006) fit Cd = 0.5 to a fired burning star; empty
            // paper shells measured 0.4 (wind tunnel) to 0.7–0.8 (free fall).
            0.5
        }
    }

    /// Semi-implicit Euler with gravity and Reynolds-dependent quadratic drag.
    /// Wind is subtracted from particle velocity before calculating drag. Drag is
    /// applied implicitly so sub-millimetre sparks, whose Stokes relaxation time
    /// approaches the 1/60 s step, stay stable.
    /// Returns distance travelled in metres.
    pub fn step(&mut self, dt: f64) -> f64 {
        if !dt.is_finite() || dt <= 0.0 {
            return 0.0;
        }
        let relative = Vec3::new(
            self.velocity.x - self.wind.x,
            self.velocity.y - self.wind.y,
            self.velocity.z - self.wind.z,
        );
        let speed = relative.length();
        // First-order barometric profile, within 0.2% of exponential below 500 m.
        let air = AIR_DENSITY * (1.0 - self.position.y.max(0.0) / SCALE_HEIGHT);
        let reynolds = air * speed * self.diameter / AIR_DYNAMIC_VISCOSITY;
        let area = std::f64::consts::PI * (self.diameter * 0.5).powi(2);
        let drag_acceleration = if speed > 0.0 && self.mass > 0.0 {
            0.5 * air * Self::drag_coefficient(reynolds) * area * speed / self.mass
        } else {
            0.0
        };
        let old_position = self.position;
        let damping = 1.0 / (1.0 + drag_acceleration * dt);
        self.velocity.x = (self.velocity.x
            + (self.acceleration.x + drag_acceleration * self.wind.x) * dt)
            * damping;
        self.velocity.y = (self.velocity.y
            + (-GRAVITY + self.acceleration.y + drag_acceleration * self.wind.y) * dt)
            * damping;
        self.velocity.z = (self.velocity.z
            + (self.acceleration.z + drag_acceleration * self.wind.z) * dt)
            * damping;
        self.position.x += self.velocity.x * dt;
        self.position.y += self.velocity.y * dt;
        self.position.z += self.velocity.z * dt;
        self.life -= dt;
        Vec3::new(
            self.position.x - old_position.x,
            self.position.y - old_position.y,
            self.position.z - old_position.z,
        )
        .length()
    }
}

#[derive(Default)]
pub struct Clock {
    remainder: f64,
}

impl Clock {
    pub const STEP_SECONDS: f64 = 1.0 / 60.0;

    /// Limit catch-up to three ticks so resuming a background tab cannot cause a stall.
    pub fn advance(&mut self, seconds: f64) -> usize {
        if !seconds.is_finite() || seconds <= 0.0 {
            return 0;
        }
        self.remainder += seconds.min(0.05);
        let steps = ((self.remainder + 1e-10) / Self::STEP_SECONDS).floor() as usize;
        self.remainder = (self.remainder - steps as f64 * Self::STEP_SECONDS).max(0.0);
        steps
    }
}

/// Contiguous, bounded storage. New emissions beyond the budget are dropped.
/// Effect code can update/remove existing items without allocating per particle.
pub struct Particles<T> {
    pub items: Vec<T>,
    capacity: usize,
}

impl<T> Particles<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn emit(&mut self, particles: impl IntoIterator<Item = T>) {
        let available = self.capacity.saturating_sub(self.items.len());
        self.items.extend(particles.into_iter().take(available));
    }

    pub fn push(&mut self, particle: T) {
        if self.items.len() < self.capacity {
            self.items.push(particle);
        }
    }

    /// Fraction of the budget still free, 0..=1.
    pub fn headroom(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            1.0 - self.items.len() as f64 / self.capacity as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrates_all_axes_with_gravity_and_drag() {
        let mut body = Body::new(Vec3::default(), Vec3::new(2.0, -2.0, 4.0), 10.0);
        body.diameter = 0.0; // isolate the exact gravity integration
        let dt = 1.0 / 60.0;
        let distance = body.step(dt);
        assert_eq!(body.velocity, Vec3::new(2.0, -2.0 - GRAVITY * dt, 4.0));
        assert_eq!(body.position.x, 2.0 * dt);
        assert_eq!(body.position.y, body.velocity.y * dt);
        assert_eq!(body.position.z, 4.0 * dt);
        assert!((body.life - (10.0 - dt)).abs() < 1e-12);
        assert!((distance - body.velocity.length() * dt).abs() < 1e-12);
    }

    #[test]
    fn quadratic_drag_slows_stars_and_wind_pushes_with_air_motion() {
        let mut still_air = Body::new(Vec3::default(), Vec3::new(100.0, 0.0, 0.0), 2.0);
        let mut tailwind = Body::new(Vec3::default(), Vec3::new(100.0, 0.0, 0.0), 2.0);
        tailwind.wind.x = 20.0;
        for _ in 0..60 {
            still_air.step(1.0 / 60.0);
            tailwind.step(1.0 / 60.0);
        }
        assert!(still_air.velocity.x < 100.0);
        assert!(tailwind.velocity.x > still_air.velocity.x);
        assert!(still_air.position.x > 0.0);
    }

    #[test]
    fn implicit_drag_keeps_tiny_sparks_stable_and_at_terminal_velocity() {
        let mut spark = Body::new(Vec3::default(), Vec3::new(0.0, -50.0, 0.0), 10.0);
        spark.wind = Vec3::default();
        spark.diameter = 0.0001;
        spark.mass = 1000.0 * std::f64::consts::PI / 6.0 * spark.diameter.powi(3);
        for _ in 0..600 {
            spark.step(1.0 / 60.0);
            assert!(spark.velocity.y.is_finite() && spark.velocity.y <= 0.0);
        }
        // Stokes terminal velocity for a 0.1 mm, 1000 kg/m³ sphere is ~0.3 m/s.
        assert!((-0.4..-0.2).contains(&spark.velocity.y));
    }

    #[test]
    fn fixed_clock_is_independent_of_render_rate() {
        for hz in [30, 60, 120, 144] {
            let mut clock = Clock::default();
            let steps: usize = (0..hz).map(|_| clock.advance(1.0 / hz as f64)).sum();
            assert_eq!(steps, 60);
        }
    }

    #[test]
    fn clock_rejects_invalid_time_and_limits_catch_up() {
        let mut clock = Clock::default();
        for dt in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(clock.advance(dt), 0);
        }
        assert_eq!(clock.advance(100.0), 3);
        assert_eq!(clock.advance(Clock::STEP_SECONDS), 1);
    }

    #[test]
    fn emissions_respect_budget_and_reuse_space() {
        let mut particles = Particles::new(2);
        particles.emit([1, 2, 3]);
        assert_eq!(particles.items, [1, 2]);
        particles.items.swap_remove(0);
        particles.emit([4, 5]);
        assert_eq!(particles.items, [2, 4]);
    }
}
