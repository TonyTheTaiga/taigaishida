use wasm_bindgen::prelude::*;

mod chemistry;
mod comet;
mod designs;
mod fleet;
mod particle;
mod projection;
mod render;
mod rng;
mod shell;
mod show;
mod star;
mod trail;
mod world;

use particle::Clock;
use projection::Camera;
use render::Frame;
use rng::Rng;
use show::{Program, Venue};
use world::World;

/// A seed for an engine created without one.
#[cfg(not(test))]
fn fresh_seed() -> u32 {
    (js_sys::Math::random() * u32::MAX as f64) as u32
}

// Native tests need no JavaScript runtime.
#[cfg(test)]
fn fresh_seed() -> u32 {
    42
}

// ─── FireworkEngine (exported) ──────────────────────────────────────

#[wasm_bindgen]
pub struct FireworkEngine {
    clock: Clock,
    /// Viewport size in CSS pixels.
    width: f64,
    height: f64,
    frame: Frame,
    world: World,
    light: fleet::Light,
    venue: Venue,
    seed: u32,
    /// How many times the programme has looped.
    loops: u64,
    program: Program,
    /// Seconds into the current loop of the programme.
    time: f64,
    /// Next cue in the programme.
    cue: usize,
}

#[wasm_bindgen]
impl FireworkEngine {
    /// `mobile` selects the portrait-first phone programme and its smaller
    /// particle budget; otherwise the panoramic desktop show plays. The same
    /// `seed` replays the same show; without one, each engine draws its own.
    #[wasm_bindgen(constructor)]
    pub fn new(width: f64, height: f64, mobile: bool, seed: Option<u32>) -> Self {
        let venue = if mobile {
            Venue::Mobile
        } else {
            Venue::Desktop
        };
        let seed = seed.unwrap_or_else(fresh_seed);
        let budget = venue.budget();
        Self {
            clock: Clock::default(),
            width,
            height,
            frame: Frame::new(
                budget.stars + budget.puffs + budget.lamps,
                budget.segments(),
                fleet::MAX_VERTICES,
            ),
            world: World::new(&budget, seed as u64),
            light: fleet::Light::default(),
            venue,
            seed,
            loops: 0,
            program: venue.program(&mut Rng::derive(seed as u64, 0)),
            time: 0.0,
            cue: 0,
        }
    }

    pub fn mobile(&self) -> bool {
        self.venue == Venue::Mobile
    }

    /// The seed this show was drawn from.
    pub fn seed(&self) -> u32 {
        self.seed
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
            self.world.fire(cue);
            self.cue += 1;
        }
        self.world.step(dt);

        // Loop the programme, with fresh random draws each time.
        if self.time > self.program.duration {
            self.time = 0.0;
            self.cue = 0;
            self.loops += 1;
            self.program = self
                .venue
                .program(&mut Rng::derive(self.seed as u64, self.loops));
        }
    }

    fn render(&mut self) {
        self.frame.clear();
        let camera = self.camera();
        let world = &self.world;
        self.light
            .gather(self.venue.fleet(), world.burning(), &world.puffs.items);
        fleet::render(
            self.venue.fleet(),
            &mut self.light,
            &camera,
            &mut self.frame,
        );
        for puff in &world.puffs.items {
            render::puff(puff, &camera, &mut self.frame);
        }
        for spark in &world.sparks.items {
            render::spark(spark, &camera, &mut self.frame);
        }
        for star in world.burning() {
            render::star(star, &camera, &mut self.frame);
        }
    }

    /// Resize to a `width` × `height` CSS-pixel viewport. The camera
    /// reframes the same physical stage, so the show itself is unaffected.
    pub fn resize(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
        self.frame.clear();
    }

    fn camera(&self) -> Camera {
        Camera::new(self.width, self.height, self.venue.stage())
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

    /// CSS pixels from the top of the viewport to the horizon, where sky
    /// meets water.
    pub fn horizon(&self) -> f64 {
        self.camera().horizon()
    }

    /// CSS pixels from the top to the water surface beneath the barges, the
    /// line bursts reflect about.
    pub fn waterline(&self) -> f64 {
        self.camera().waterline()
    }

    /// Live particles for the on-screen counter: burning stars (and comets
    /// riding on climbing shells), spark particles, smoke and flash parcels,
    /// and shells still in flight.
    pub fn star_count(&self) -> u32 {
        self.world.burning().count() as u32
    }

    pub fn spark_count(&self) -> u32 {
        self.world.sparks.items.len() as u32
    }

    pub fn smoke_count(&self) -> u32 {
        self.world.puffs.items.len() as u32
    }

    pub fn shell_count(&self) -> u32 {
        self.world.shells.len() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use designs::{ALL, STAR_MINE};
    use particle::Vec3;
    use render::{POINT_STRIDE, TRAIL_STRIDE};
    use shell::{Fuse, Shell, ShellDesign};
    use show::Cue;

    const DESKTOP: show::Budget = Venue::Desktop.budget();

    /// The signature shells and the star-mine shells.
    fn every_design() -> impl Iterator<Item = &'static &'static ShellDesign> {
        ALL.iter().chain(STAR_MINE)
    }

    /// Burst a design at altitude and run it to completion, returning the
    /// widest horizontal spread of the flower (the 90th-percentile lit star,
    /// so one outlier from an uneven burst cannot set it), the deepest fall,
    /// the time to burn out, and the peak spark count.
    fn burst_alone(design: &'static ShellDesign) -> (f64, f64, f64, usize) {
        let mut world = World::new(&DESKTOP, 42);
        let shell = Shell::new(
            design,
            Vec3::new(0.0, 250.0, 0.0),
            Vec3::default(),
            Fuse::Burning(0.0),
            &mut world.rng,
        );
        world.launch(shell);
        let mut reach: f64 = 0.0;
        let mut drop: f64 = 0.0;
        let mut time = 0.0;
        let mut peak_sparks = 0;
        let mut spreads = Vec::new();
        while time < 20.0 {
            world.step(Clock::STEP_SECONDS);
            peak_sparks = peak_sparks.max(world.sparks.items.len());
            spreads.clear();
            for star in &world.stars.items {
                let p = star.body.position;
                assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
                if star.light().is_some_and(|(_, l)| l > 0.0) {
                    spreads.push(p.x.hypot(p.z));
                    drop = drop.max(250.0 - p.y);
                }
            }
            if !spreads.is_empty() {
                let k = spreads.len() * 9 / 10;
                let (_, ninetieth, _) = spreads.select_nth_unstable_by(k, f64::total_cmp);
                reach = reach.max(*ninetieth);
            }
            time += Clock::STEP_SECONDS;
            if world.is_dark() {
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
            let mut world = World::new(&DESKTOP, 7);
            world.fire(&Cue::shell(0.0, design, Vec3::new(40.0, 0.0, 0.0), 0.0));
            let muzzle = world.shells[0].body.velocity.length();
            let mut rising = muzzle;
            let mut fuse_sparks = false;
            let mut time = 0.0;
            while world.bursts.is_empty() && !world.shells.is_empty() {
                rising = world.shells[0].body.velocity.y;
                world.step(Clock::STEP_SECONDS);
                fuse_sparks |= !world.sparks.items.is_empty();
                time += Clock::STEP_SECONDS;
            }
            assert!(!world.bursts.is_empty(), "{} lost", design.name);
            let burst = world.bursts[0];
            let (height, _, _) = published(design.diameter);
            let altitude = burst.y;
            println!(
                "{:<24} {:>4.2} kg  muzzle {muzzle:>5.1} m/s  burst {altitude:>5.1} m (published {height})  after {time:.1} s",
                design.name,
                design.mass()
            );
            // Lift charges give about 112 m/s whatever the size (Kosanke);
            // a 5-gō shell measured 138 m/s, and a 3-gō needs only 86 m/s
            // to reach the Association's 125 m. Each shot scatters by 3%.
            let nominal = design.muzzle_speed();
            assert!(
                (80.0..=150.0).contains(&nominal),
                "{} muzzle {nominal}",
                design.name
            );
            assert!((muzzle / nominal - 1.0).abs() <= 0.031);
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
            assert!((burst.x - 40.0).abs() < 15.0, "{}", design.name);
            assert!(rising.abs() < 5.0, "{} bursts at its apex", design.name);
            assert!(fuse_sparks, "the time fuse leaves sparks on the way up");
        }
    }

    #[test]
    fn extreme_viewports_run_without_panicking() {
        for (width, height) in [
            (308.0, 1386.0),
            (700.0, 3240.0),
            (1.0, 9000.0),
            (7000.0, 1.0),
            (0.0, 0.0),
            (28000.0, 36000.0),
        ] {
            for mobile in [false, true] {
                let mut engine = FireworkEngine::new(width, height, mobile, None);
                engine.tick(1.0);
                engine.resize(height, width);
                engine.tick(1.0);
            }
        }
    }

    #[test]
    fn both_shows_stay_within_budget_finite_and_loop() {
        for (mobile, width, height) in [(false, 1400.0, 1080.0), (true, 390.0, 844.0)] {
            let mut engine = FireworkEngine::new(width, height, mobile, Some(9));
            let budget = engine.venue.budget();
            let duration = engine.program.duration;
            let mut rendered = false;
            let mut has_depth = false;
            let mut peak = (0, 0, 0);
            for _ in 0..((duration as usize + 5) * 60) {
                engine.tick(Clock::STEP_SECONDS);
                let world = &engine.world;
                has_depth |= world.stars.items.iter().any(|s| s.body.velocity.z != 0.0);
                peak.0 = peak.0.max(world.stars.items.len());
                peak.1 = peak.1.max(world.sparks.items.len());
                peak.2 = peak.2.max(engine.trails_len() / TRAIL_STRIDE);
                assert!(world.stars.items.len() <= budget.stars);
                assert!(world.sparks.items.len() <= budget.sparks);
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
        let mut engine = FireworkEngine::new(1400.0, 1080.0, false, Some(5));
        for _ in 0..30 * 60 {
            engine.tick(Clock::STEP_SECONDS);
        }
        let launched = engine.cue;
        let shells = engine.world.shells.len();
        engine.resize(560.0, 1440.0);
        assert!(engine.waterline() > engine.horizon());
        assert!((engine.horizon() - 1440.0 * projection::SKY_FRACTION).abs() < 1e-6);
        assert_eq!(engine.cue, launched);
        assert_eq!(engine.world.shells.len(), shells);
        assert!(engine.program.cues[..launched]
            .iter()
            .all(|cue| cue.time <= engine.time));
        engine.resize(0.0, 0.0);
        engine.tick(Clock::STEP_SECONDS);
        assert!(engine.horizon().is_finite());
    }

    #[test]
    fn a_seed_replays_the_same_show_and_engines_are_independent() {
        let frames = |seed: u32| {
            let mut engine = FireworkEngine::new(1400.0, 1080.0, false, Some(seed));
            for _ in 0..12 * 60 {
                engine.tick(Clock::STEP_SECONDS);
            }
            assert_eq!(engine.seed(), seed);
            engine.frame.points.clone()
        };
        let first = frames(11);
        // Another engine running in between draws from its own dice.
        let _ = frames(12);
        assert_eq!(first, frames(11));
        assert_ne!(first, frames(12));
    }
}
