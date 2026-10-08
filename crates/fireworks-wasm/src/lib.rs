use wasm_bindgen::prelude::*;

mod chemistry;
mod comet;
mod designs;
mod fleet;
mod particle;
mod projection;
mod render;
mod shell;
mod show;
mod star;
mod trail;

use particle::{Clock, Particles, Vec3};
use projection::Camera;
use render::Frame;
use shell::{Burst, Flight, Shell};
use show::{Program, Venue};
use star::{Emission, Puff, Spark, Star};

// ─── Constants ──────────────────────────────────────────────────────

/// Share of stars whose burnt-out smoke is tracked as its own parcel. Each
/// stands for the smoke of several stars, so a burst leaves a faint shell of
/// smoke the size of its flower.
const STAR_SMOKE_SHARE: f64 = 0.12;

// ─── RNG helpers ────────────────────────────────────────────────────

/// PCG-style generator kept inside WASM: sparks draw random numbers every
/// tick, and crossing into JavaScript for each one dominated simulation time.
fn rand_f64() -> f64 {
    use std::cell::Cell;
    thread_local! { static STATE: Cell<u64> = Cell::new(initial_seed()); }
    STATE.with(|state| {
        let next = state
            .get()
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        state.set(next);
        (next >> 11) as f64 / ((1u64 << 53) as f64)
    })
}

#[cfg(not(test))]
fn initial_seed() -> u64 {
    (js_sys::Math::random() * u64::MAX as f64) as u64 | 1
}

// Native tests are deterministic and need no JavaScript runtime.
#[cfg(test)]
fn initial_seed() -> u64 {
    42
}

pub(crate) fn rand(min: f64, max: f64) -> f64 {
    rand_f64() * (max - min) + min
}

pub(crate) fn pick<T: Copy>(arr: &[T]) -> T {
    arr[(rand_f64() * arr.len() as f64).floor() as usize % arr.len()]
}

/// Standard normal deviate (Box–Muller).
pub(crate) fn gauss() -> f64 {
    let u = rand_f64().max(1e-12);
    let v = rand_f64();
    (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
}

/// Uniformly distributed direction on the unit sphere.
pub(crate) fn random_unit() -> Vec3 {
    let z = rand(-1.0, 1.0);
    let a = rand(0.0, std::f64::consts::TAU);
    let r = (1.0 - z * z).sqrt();
    Vec3::new(r * a.cos(), r * a.sin(), z)
}

// ─── FireworkEngine (exported) ──────────────────────────────────────

#[wasm_bindgen]
pub struct FireworkEngine {
    clock: Clock,
    cols: usize,
    rows: usize,
    frame: Frame,
    shells: Vec<Shell>,
    stars: Particles<Star>,
    sparks: Particles<Spark>,
    puffs: Particles<Puff>,
    pending_stars: Vec<Star>,
    pending_shells: Vec<Shell>,
    light: fleet::Light,
    venue: Venue,
    program: Program,
    /// Seconds into the current loop of the programme.
    time: f64,
    /// Next cue in the programme.
    cue: usize,
}

#[wasm_bindgen]
impl FireworkEngine {
    /// `mobile` selects the portrait-first phone programme and its smaller
    /// particle budget; otherwise the panoramic desktop show plays.
    #[wasm_bindgen(constructor)]
    pub fn new(cols: u32, rows: u32, mobile: bool) -> Self {
        let venue = if mobile {
            Venue::Mobile
        } else {
            Venue::Desktop
        };
        let budget = venue.budget();
        Self {
            clock: Clock::default(),
            cols: cols as usize,
            rows: rows as usize,
            frame: Frame::new(
                budget.stars + budget.puffs + budget.lamps,
                budget.segments(),
                fleet::MAX_VERTICES,
            ),
            shells: Vec::new(),
            stars: Particles::new(budget.stars),
            sparks: Particles::new(budget.sparks),
            puffs: Particles::new(budget.puffs),
            pending_stars: Vec::new(),
            pending_shells: Vec::new(),
            light: fleet::Light::default(),
            venue,
            program: venue.program(),
            time: 0.0,
            cue: 0,
        }
    }

    pub fn mobile(&self) -> bool {
        self.venue == Venue::Mobile
    }

    pub fn tick(&mut self, dt_sec: f64) {
        for _ in 0..self.clock.advance(dt_sec) {
            self.step();
        }
        self.render();
    }

    fn step(&mut self) {
        let dt = Clock::STEP_SECONDS;
        self.time += dt;
        while let Some(cue) = self
            .program
            .cues
            .get(self.cue)
            .filter(|cue| cue.time <= self.time)
        {
            cue.fire(&mut self.shells, &mut self.pending_stars, &mut self.puffs);
            self.cue += 1;
        }

        // Thin spark sampling once half the budget is in use, rather than
        // letting whichever star updates first claim every slot.
        let spark_rate = (self.sparks.headroom() * 2.0).min(1.0);
        let mut emission = Emission {
            sparks: &mut self.sparks,
            stars: &mut self.pending_stars,
            spark_rate,
        };

        let mut i = 0;
        while i < self.shells.len() {
            match self.shells[i].update(dt, &mut emission) {
                Flight::Climbing => i += 1,
                Flight::Burst => {
                    let mut shell = self.shells.swap_remove(i);
                    shell.burst(&mut Burst {
                        stars: emission.stars,
                        shells: &mut self.pending_shells,
                        puffs: &mut self.puffs,
                    });
                }
                Flight::Lost => {
                    self.shells.swap_remove(i);
                }
            }
        }
        self.shells.append(&mut self.pending_shells);

        let mut i = 0;
        while i < self.stars.items.len() {
            if self.stars.items[i].update(dt, &mut emission) {
                i += 1;
            } else {
                let star = self.stars.items.swap_remove(i);
                let p = star.body.position;
                if star.radius() <= 0.0 && p.y > 0.0 && rand_f64() < STAR_SMOKE_SHARE {
                    self.puffs.push(Puff::smoke(
                        p,
                        star.body.velocity.scale(0.3),
                        rand(2.5, 4.5),
                        0.12,
                        rand(14.0, 22.0),
                    ));
                }
            }
        }
        self.stars.emit(self.pending_stars.drain(..));
        self.sparks.items.retain_mut(|spark| spark.update(dt));
        self.puffs.items.retain_mut(|puff| puff.update(dt));

        // Loop the programme, with fresh random draws each time.
        if self.time > self.program.duration {
            self.time = 0.0;
            self.cue = 0;
            self.program = self.venue.program();
        }
    }

    fn render(&mut self) {
        self.frame.clear();
        let camera = Camera::new(self.cols, self.rows, self.venue.stage());
        let attached = self
            .shells
            .iter()
            .flat_map(|shell| shell.attached.iter().flatten());
        self.light.gather(
            self.venue.fleet(),
            self.stars.items.iter().chain(attached.clone()),
            &self.puffs.items,
        );
        fleet::render(
            self.venue.fleet(),
            &mut self.light,
            &camera,
            &mut self.frame,
        );
        for puff in &self.puffs.items {
            render::puff(puff, &camera, &mut self.frame);
        }
        for spark in &self.sparks.items {
            render::spark(spark, &camera, &mut self.frame);
        }
        for star in self.stars.items.iter().chain(attached) {
            render::star(star, &camera, &mut self.frame);
        }
    }

    pub fn resize(&mut self, cols: u32, rows: u32) {
        // The camera reframes the same physical stage, so the show itself is
        // unaffected by the viewport.
        self.cols = cols as usize;
        self.rows = rows as usize;
        self.frame.clear();
    }

    /// Packed point data for smooth renderers. Lengths are in f32 elements,
    /// not bytes; `render.rs` documents each layout.
    pub fn points_ptr(&self) -> *const f32 {
        self.frame.points.as_ptr()
    }

    pub fn points_len(&self) -> usize {
        self.frame.points.len()
    }

    pub fn trails_ptr(&self) -> *const f32 {
        self.frame.trails.as_ptr()
    }

    pub fn trails_len(&self) -> usize {
        self.frame.trails.len()
    }

    pub fn mesh_ptr(&self) -> *const f32 {
        self.frame.mesh.as_ptr()
    }

    pub fn mesh_len(&self) -> usize {
        self.frame.mesh.len()
    }

    /// Grid row of the horizon, where sky meets water.
    pub fn horizon(&self) -> f64 {
        Camera::new(self.cols, self.rows, self.venue.stage()).horizon()
    }

    /// Grid row of the water surface beneath the barges, the line bursts
    /// reflect about.
    pub fn waterline(&self) -> f64 {
        Camera::new(self.cols, self.rows, self.venue.stage()).waterline()
    }

    /// Live particles for the on-screen counter: burning stars (and comets
    /// riding on climbing shells), spark particles, smoke and flash parcels,
    /// and shells still in flight.
    pub fn star_count(&self) -> u32 {
        let attached: usize = self
            .shells
            .iter()
            .map(|shell| shell.attached.iter().flatten().count())
            .sum();
        (self.stars.items.len() + attached) as u32
    }

    pub fn spark_count(&self) -> u32 {
        self.sparks.items.len() as u32
    }

    pub fn smoke_count(&self) -> u32 {
        self.puffs.items.len() as u32
    }

    pub fn shell_count(&self) -> u32 {
        self.shells.len() as u32
    }

    pub fn cols(&self) -> u32 {
        self.cols as u32
    }

    pub fn rows(&self) -> u32 {
        self.rows as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use designs::{ALL, STAR_MINE};
    use render::{POINT_STRIDE, TRAIL_STRIDE};
    use shell::ShellDesign;
    use show::Cue;

    const DESKTOP: show::Budget = Venue::Desktop.budget();

    /// The signature shells and the star-mine shells.
    fn every_design() -> impl Iterator<Item = &'static &'static ShellDesign> {
        ALL.iter().chain(STAR_MINE)
    }

    /// Burst a design at altitude and run it to completion, returning the
    /// widest horizontal spread of lit stars, their deepest fall, the time to
    /// burn out, and the peak spark count.
    fn burst_alone(design: &'static ShellDesign) -> (f64, f64, f64, usize) {
        let mut shells = vec![Shell::new(
            design,
            Vec3::new(0.0, 250.0, 0.0),
            Vec3::default(),
            shell::Fuse::Burning(0.0),
        )];
        let mut stars: Particles<Star> = Particles::new(DESKTOP.stars);
        let mut sparks = Particles::new(DESKTOP.sparks);
        let mut puffs = Particles::new(DESKTOP.puffs);
        let mut pending = Vec::new();
        let mut reach: f64 = 0.0;
        let mut drop: f64 = 0.0;
        let mut time = 0.0;
        let mut peak_sparks = 0;
        while time < 20.0 {
            let spark_rate = (sparks.headroom() * 2.0).min(1.0);
            let mut emission = Emission {
                sparks: &mut sparks,
                stars: &mut pending,
                spark_rate,
            };
            let mut new_shells = Vec::new();
            shells.retain_mut(
                |shell| match shell.update(Clock::STEP_SECONDS, &mut emission) {
                    Flight::Burst => {
                        shell.burst(&mut Burst {
                            stars: emission.stars,
                            shells: &mut new_shells,
                            puffs: &mut puffs,
                        });
                        false
                    }
                    Flight::Climbing => true,
                    Flight::Lost => false,
                },
            );
            shells.extend(new_shells);
            stars
                .items
                .retain_mut(|star| star.update(Clock::STEP_SECONDS, &mut emission));
            stars.emit(pending.drain(..));
            sparks.items.retain_mut(|s| s.update(Clock::STEP_SECONDS));
            peak_sparks = peak_sparks.max(sparks.items.len());
            for star in &stars.items {
                let p = star.body.position;
                assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
                if star.light().is_some_and(|(_, l)| l > 0.0) {
                    reach = reach.max(p.x.hypot(p.z));
                    drop = drop.max(250.0 - p.y);
                }
            }
            time += Clock::STEP_SECONDS;
            if shells.is_empty() && stars.items.is_empty() && sparks.items.is_empty() {
                break;
            }
        }
        (reach, drop, time, peak_sparks)
    }

    /// Published figures by shell diameter: the Japan Fireworks Association
    /// burst height, and the flower diameter from Shimizu's Table 2 (low end)
    /// to the Association's table (high end).
    fn published(diameter: f64) -> (f64, f64, f64) {
        match (diameter * 1000.0).round() as u32 {
            86 => (125.0, 60.0, 70.0),
            114 => (165.0, 110.0, 120.0),
            142 => (195.0, 100.0, 160.0),
            171 => (220.0, 130.0, 200.0),
            199 => (250.0, 170.0, 220.0),
            other => panic!("no published data for a {other} mm shell"),
        }
    }

    fn is_warimono(design: &ShellDesign) -> bool {
        design.burst_heat == chemistry::PERCHLORATE_BURST_HEAT
    }

    #[test]
    fn every_design_bursts_to_its_published_size_and_burns_out() {
        assert_eq!(ALL.len(), 10);
        for design in every_design() {
            let speed = design.burst_speed();
            let (reach, drop, time, peak_sparks) = burst_alone(design);
            let (_, small, large) = published(design.diameter);
            println!(
                "{:<24} {:>4.2} kg  burst {:>5.1} m/s  flower {:>5.1} m ({small}–{large})  fall {:>5.1} m  lasts {:>4.1} s  sparks {:>5}",
                design.name,
                design.mass(),
                speed,
                2.0 * reach,
                drop,
                time,
                peak_sparks
            );
            // Shimizu (Table 19) measured 57–71 m/s from 6-inch bursts.
            if is_warimono(design) {
                assert!(
                    (50.0..=75.0).contains(&speed),
                    "{} burst speed {speed}",
                    design.name
                );
            }
            // The published diameters are typical figures. Slow-burning stars
            // fly further: violet and blue peonies open up to a third wider
            // than red, as Ooki's violet and blue stars outlast silver ones.
            // Poka shells open gently and their spread depends on what the
            // stars do afterwards (fish wander at random).
            let low = if is_warimono(design) { 0.75 } else { 0.6 };
            assert!(
                (low * small..=1.35 * large).contains(&(2.0 * reach)),
                "{} flower {} m",
                design.name,
                2.0 * reach
            );
            // A 9 mm peony star lasts 1.5 s; a kamuro crown more than ten.
            assert!((1.0..25.0).contains(&time), "{} lasted {time}", design.name);
        }
    }

    #[test]
    fn lift_carries_each_shell_to_its_published_height_and_fuses_at_the_apex() {
        for design in every_design() {
            let mut shells = Vec::new();
            let mut stars = Vec::new();
            let mut puffs = Particles::new(10);
            Cue::shell(0.0, design, Vec3::new(40.0, 0.0, 0.0), 0.0).fire(
                &mut shells,
                &mut stars,
                &mut puffs,
            );
            let mut shell = shells.pop().unwrap();
            let muzzle = shell.body.velocity.length();
            let mut sparks = Particles::new(DESKTOP.sparks);
            let mut pending = Vec::new();
            let mut emission = Emission {
                sparks: &mut sparks,
                stars: &mut pending,
                spark_rate: 1.0,
            };
            let mut flight = Flight::Climbing;
            let mut time = 0.0;
            while matches!(flight, Flight::Climbing) {
                flight = shell.update(Clock::STEP_SECONDS, &mut emission);
                time += Clock::STEP_SECONDS;
            }
            let (height, _, _) = published(design.diameter);
            let altitude = shell.body.position.y;
            println!(
                "{:<24} {:>4.2} kg  muzzle {muzzle:>5.1} m/s  burst {altitude:>5.1} m (published {height})  after {time:.1} s",
                design.name,
                design.mass()
            );
            assert!(matches!(flight, Flight::Burst), "{} lost", design.name);
            // Lift charges give about 112 m/s whatever the size (Kosanke);
            // a 5-gō shell measured 138 m/s, and a 3-gō needs only 86 m/s
            // to reach the Association's 125 m.
            assert!(
                (80.0..=150.0).contains(&muzzle),
                "{} muzzle {muzzle}",
                design.name
            );
            // Standard warimono match the table; lighter poka shells, opened
            // by a few grams of powder, carry less momentum against drag.
            let band = if is_warimono(design) {
                0.88..=1.12
            } else {
                0.6..=1.12
            };
            assert!(
                band.contains(&(altitude / height)),
                "{} burst at {altitude} m",
                design.name
            );
            // Kosanke: time fuses burn 3–6 s, more for larger shells.
            assert!((3.0..=7.5).contains(&time), "{} took {time} s", design.name);
            assert!(
                (shell.body.position.x - 40.0).abs() < 15.0,
                "{}",
                design.name
            );
            assert!(shell.body.velocity.y.abs() < 5.0);
            assert!(!sparks.items.is_empty(), "the time fuse leaves sparks");
        }
    }

    #[test]
    fn extreme_viewports_run_without_panicking() {
        for (cols, rows) in [
            (22, 77),
            (50, 180),
            (1, 500),
            (500, 1),
            (0, 0),
            (2000, 2000),
        ] {
            for mobile in [false, true] {
                let mut engine = FireworkEngine::new(cols as u32, rows as u32, mobile);
                engine.tick(1.0);
                engine.resize(rows as u32, cols as u32);
                engine.tick(1.0);
            }
        }
    }

    #[test]
    fn both_shows_stay_within_budget_finite_and_loop() {
        for (mobile, cols, rows) in [(false, 100, 60), (true, 27, 46)] {
            let mut engine = FireworkEngine::new(cols, rows, mobile);
            let budget = engine.venue.budget();
            let duration = engine.program.duration;
            let mut rendered = false;
            let mut has_depth = false;
            let mut peak = (0, 0, 0);
            for _ in 0..((duration as usize + 5) * 60) {
                engine.tick(Clock::STEP_SECONDS);
                has_depth |= engine.stars.items.iter().any(|s| s.body.velocity.z != 0.0);
                peak.0 = peak.0.max(engine.stars.items.len());
                peak.1 = peak.1.max(engine.sparks.items.len());
                peak.2 = peak.2.max(engine.trails_len() / TRAIL_STRIDE);
                assert!(engine.stars.items.len() <= budget.stars);
                assert!(engine.sparks.items.len() <= budget.sparks);
                assert_eq!(engine.points_len() % POINT_STRIDE, 0);
                assert_eq!(engine.trails_len() % TRAIL_STRIDE, 0);
                assert_eq!(engine.mesh_len() % (render::MESH_STRIDE * 3), 0);
                assert!(engine.trails_len() <= budget.segments() * TRAIL_STRIDE);
                assert!(engine.mesh_len() <= fleet::MAX_VERTICES * render::MESH_STRIDE);
                let frame = &engine.frame;
                assert!(frame.trails.iter().all(|v| v.is_finite()));
                assert!(frame.points.iter().all(|v| v.is_finite()));
                assert!(frame.mesh.iter().all(|v| v.is_finite() && *v >= 0.0));
                for point in frame.points.chunks_exact(POINT_STRIDE) {
                    assert!(point[2] > 0.0);
                    assert!(point[6] >= 0.0);
                }
                rendered |= !frame.points.is_empty();
            }
            println!(
                "{:?}: peak stars {} sparks {} segments {}",
                engine.venue, peak.0, peak.1, peak.2
            );
            // The star budget must never clip a burst.
            assert!(peak.0 < budget.stars, "{:?} ran out of stars", engine.venue);
            assert!(rendered);
            assert!(has_depth);
            assert!(engine.time < 6.0, "show must loop");
            assert!(engine.waterline() > engine.horizon());
            assert_eq!(engine.mobile(), mobile);
        }
    }

    #[test]
    fn resizing_does_not_replay_past_launches() {
        let mut engine = FireworkEngine::new(100, 60, false);
        for _ in 0..30 * 60 {
            engine.tick(Clock::STEP_SECONDS);
        }
        let launched = engine.cue;
        let shells = engine.shells.len();
        engine.resize(40, 80);
        assert_eq!(engine.cols(), 40);
        assert_eq!(engine.rows(), 80);
        assert_eq!(engine.cue, launched);
        assert_eq!(engine.shells.len(), shells);
        assert!(engine.program.cues[..launched]
            .iter()
            .all(|cue| cue.time <= engine.time));
        engine.resize(0, 0);
        engine.tick(Clock::STEP_SECONDS);
        assert_eq!(engine.cols(), 0);
        assert_eq!(engine.rows(), 0);
    }
}
