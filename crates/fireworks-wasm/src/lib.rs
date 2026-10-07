use std::sync::OnceLock;

use wasm_bindgen::prelude::*;

mod chemistry;
mod designs;
mod particle;
mod projection;
mod shell;
mod show;
mod star;
mod trail;

use chemistry::{blackbody, perceived, Rgb};
use particle::{Clock, Particles, Vec3};
use projection::Camera;
use shell::{Burst, Flight, Shell};
use show::{Program, Venue};
use star::{Emission, Puff, PuffKind, Spark, Star};

// ─── Constants ──────────────────────────────────────────────────────

/// Stars draw four history segments and sparks one; the desktop budget is the
/// largest either venue can produce.
const MAX_TRAIL_SEGMENTS: usize = {
    let budget = Venue::Desktop.budget();
    budget.stars * 4 + budget.sparks
};
/// Persistence of vision: how long a moving spark smears across the retina.
const EXPOSURE: f64 = 0.08;

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

/// Uniformly distributed direction on the unit sphere.
pub(crate) fn random_unit() -> Vec3 {
    let z = rand(-1.0, 1.0);
    let a = rand(0.0, std::f64::consts::TAU);
    let r = (1.0 - z * z).sqrt();
    Vec3::new(r * a.cos(), r * a.sin(), z)
}

// ─── Light output ───────────────────────────────────────────────────

const LUT_MIN: f64 = 800.0;
const LUT_STEP: f64 = 25.0;
const LUT_SIZE: usize = 140;

/// Blackbody colours from 800 K to 4275 K, computed once.
fn spark_colour(temperature: f64) -> Rgb {
    static TABLE: OnceLock<Vec<Rgb>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        (0..LUT_SIZE)
            .map(|i| blackbody(LUT_MIN + i as f64 * LUT_STEP))
            .collect()
    });
    let index = ((temperature - LUT_MIN) / LUT_STEP)
        .round()
        .clamp(0.0, (LUT_SIZE - 1) as f64);
    table[index as usize]
}

fn write_point(
    points: &mut Vec<f32>,
    at: (f64, f64),
    radius: f64,
    rgb: Rgb,
    alpha: f64,
    kind: f32,
) {
    points.extend_from_slice(&[
        at.0 as f32,
        at.1 as f32,
        radius as f32,
        rgb.0 * 255.0,
        rgb.1 * 255.0,
        rgb.2 * 255.0,
        alpha.clamp(0.0, 1.0) as f32,
        kind,
    ]);
}

fn write_segment(
    trails: &mut Vec<f32>,
    from: (f64, f64),
    to: (f64, f64),
    width: f64,
    rgb: Rgb,
    alpha: f64,
) {
    trails.extend_from_slice(&[
        from.0 as f32,
        from.1 as f32,
        to.0 as f32,
        to.1 as f32,
        width as f32,
        rgb.0 * 255.0,
        rgb.1 * 255.0,
        rgb.2 * 255.0,
        alpha.clamp(0.0, 1.0) as f32,
        0.0,
    ]);
}

/// Distant light dims and shrinks; it never vanishes entirely.
fn distance_dimming(ratio: f64) -> f64 {
    ratio.clamp(0.35, 1.0)
}

fn render_star(star: &Star, camera: &Camera, points: &mut Vec<f32>, trails: &mut Vec<f32>) {
    let Some((rgb, luminance)) = star.light() else {
        return;
    };
    let brightness = perceived(luminance);
    if brightness < 0.02 {
        return;
    }
    let p = star.body.position;
    let Some(mut head) = camera.project(p.x, p.y, p.z) else {
        return;
    };
    let ratio = head.2;
    let alpha = brightness.min(1.0) * distance_dimming(ratio);
    let radius = (0.7 + 1.1 * brightness.min(2.0)) * ratio.clamp(0.3, 3.0);
    write_point(points, (head.0, head.1), radius, rgb, alpha, 0.0);
    // The eye integrates the last ~0.13 s of motion into a short streak.
    for (age, tail) in star
        .trail
        .samples()
        .enumerate()
        .filter(|(age, _)| age % 2 == 1)
    {
        if trails.len() / 10 >= MAX_TRAIL_SEGMENTS {
            return;
        }
        let Some(end) = camera.project(tail.x, tail.y, tail.z) else {
            return;
        };
        let fade = (1.0 - age as f64 / 9.0).powi(2);
        write_segment(
            trails,
            (head.0, head.1),
            (end.0, end.1),
            1.8 * ratio,
            rgb,
            alpha * fade * 0.85,
        );
        head = end;
    }
}

fn render_spark(spark: &Spark, camera: &Camera, points: &mut Vec<f32>, trails: &mut Vec<f32>) {
    let brightness = perceived(spark.luminance());
    if brightness < 0.03 {
        return;
    }
    let p = spark.body.position;
    let Some(head) = camera.project(p.x, p.y, p.z) else {
        return;
    };
    let rgb = spark_colour(spark.temperature);
    let ratio = head.2;
    let alpha = brightness.min(1.0) * distance_dimming(ratio);
    if trails.len() / 10 < MAX_TRAIL_SEGMENTS {
        let t = p.sub(spark.body.velocity.scale(EXPOSURE));
        if let Some(tail) = camera.project(t.x, t.y, t.z) {
            let width = (1.0 + 0.7 * brightness.min(2.0)) * ratio;
            write_segment(
                trails,
                (head.0, head.1),
                (tail.0, tail.1),
                width,
                rgb,
                alpha,
            );
        }
    }
    // Glitter flashes and popping microstars outshine their own streak.
    if brightness > 1.3 {
        let radius = (0.3 + 0.45 * brightness.min(3.0)) * ratio.clamp(0.3, 3.0);
        write_point(points, (head.0, head.1), radius, rgb, alpha, 0.0);
    }
}

fn render_puff(puff: &Puff, camera: &Camera, points: &mut Vec<f32>) {
    let p = puff.body.position;
    let Some((x, y, ratio)) = camera.project(p.x, p.y, p.z) else {
        return;
    };
    let fraction = puff.fraction();
    match puff.kind {
        PuffKind::Smoke => write_point(
            points,
            (x, y),
            12.0 * ratio.clamp(0.3, 3.0),
            Rgb(0.31, 0.29, 0.27),
            fraction.min(0.18),
            1.0,
        ),
        PuffKind::Flash => write_point(
            points,
            (x, y),
            (10.0 + 14.0 * puff.intensity) * ratio.clamp(0.3, 3.0),
            Rgb(1.0, 0.94, 0.78),
            fraction.powi(3) * puff.intensity,
            2.0,
        ),
    }
}

// ─── FireworkEngine (exported) ──────────────────────────────────────

#[wasm_bindgen]
pub struct FireworkEngine {
    clock: Clock,
    cols: usize,
    rows: usize,
    points: Vec<f32>,
    trails: Vec<f32>,
    shells: Vec<Shell>,
    stars: Particles<Star>,
    sparks: Particles<Spark>,
    puffs: Particles<Puff>,
    pending_stars: Vec<Star>,
    pending_shells: Vec<Shell>,
    venue: Venue,
    program: Program,
    /// Seconds into the current loop of the programme.
    time: f64,
    /// Next launch in the programme.
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
            points: Vec::with_capacity((budget.stars + budget.puffs) * 8),
            trails: Vec::with_capacity((budget.stars * 4 + budget.sparks) * 10),
            shells: Vec::new(),
            stars: Particles::new(budget.stars),
            sparks: Particles::new(budget.sparks),
            puffs: Particles::new(budget.puffs),
            pending_stars: Vec::new(),
            pending_shells: Vec::new(),
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
        while let Some(launch) = self
            .program
            .launches
            .get(self.cue)
            .filter(|launch| launch.time <= self.time)
        {
            self.shells.push(launch.fire());
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
                self.stars.items.swap_remove(i);
            }
        }
        self.stars.emit(self.pending_stars.drain(..));
        self.sparks.items.retain_mut(|spark| spark.update(dt));
        self.puffs.items.retain_mut(|puff| puff.update(dt));

        // Loop the programme, with fresh star-mine draws each time.
        if self.time > self.program.duration {
            self.time = 0.0;
            self.cue = 0;
            self.program = self.venue.program();
        }
    }

    fn render(&mut self) {
        self.points.clear();
        self.trails.clear();
        let camera = Camera::new(self.cols, self.rows, self.venue.stage());
        for puff in &self.puffs.items {
            render_puff(puff, &camera, &mut self.points);
        }
        for spark in &self.sparks.items {
            render_spark(spark, &camera, &mut self.points, &mut self.trails);
        }
        for star in self.stars.items.iter().chain(
            self.shells
                .iter()
                .flat_map(|shell| shell.attached.iter().flatten()),
        ) {
            render_star(star, &camera, &mut self.points, &mut self.trails);
        }
    }

    pub fn resize(&mut self, cols: u32, rows: u32) {
        let cols = cols as usize;
        let rows = rows as usize;
        self.cols = cols;
        self.rows = rows;
        // The camera reframes the same physical stage, so the show itself is
        // unaffected by the viewport.
        self.points.clear();
        self.trails.clear();
    }

    /// Packed point data for smooth renderers. Length is in f32 elements, not bytes.
    pub fn points_ptr(&self) -> *const f32 {
        self.points.as_ptr()
    }

    pub fn points_len(&self) -> usize {
        self.points.len()
    }

    pub fn trails_ptr(&self) -> *const f32 {
        self.trails.as_ptr()
    }

    pub fn trails_len(&self) -> usize {
        self.trails.len()
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
    use shell::ShellDesign;
    use show::Launch;

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
            0.0,
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
            assert!(
                (0.75 * small..=1.35 * large).contains(&(2.0 * reach)),
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
            let mut shell = Launch::new(0.0, design, 40.0, 0.0).fire();
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
                peak.2 = peak.2.max(engine.trails_len() / 10);
                assert!(engine.stars.items.len() <= budget.stars);
                assert!(engine.sparks.items.len() <= budget.sparks);
                assert_eq!(engine.points_len() % 8, 0);
                assert_eq!(engine.trails_len() % 10, 0);
                assert!(engine.trails_len() <= (budget.stars * 4 + budget.sparks) * 10);
                assert!(engine.trails.iter().all(|v| v.is_finite()));
                assert!(engine.points.iter().all(|v| v.is_finite()));
                for point in engine.points.chunks_exact(8) {
                    assert!(point[2] > 0.0);
                    assert!((0.0..=1.0).contains(&point[6]));
                }
                rendered |= !engine.points.is_empty();
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
            assert_eq!(engine.mobile(), mobile);
        }
    }

    #[test]
    fn spark_and_star_output_match_their_physical_light() {
        let camera = Camera::new(100, 60, Venue::Desktop.stage());
        let mut points = Vec::new();
        let mut trails = Vec::new();
        const RECIPE: &[chemistry::Layer] = &[chemistry::Layer {
            composition: &chemistry::STRONTIUM_RED,
            thickness: 0.004,
        }];
        let star = Star::new(
            RECIPE,
            1.0,
            Vec3::new(10.25, 120.0, 15.0),
            Vec3::new(5.0, 0.0, 0.0),
        );
        render_star(&star, &camera, &mut points, &mut trails);
        assert_eq!(points.len(), 8);
        let (x, y, _) = camera.project(10.25, 120.0, 15.0).unwrap();
        assert_eq!(&points[..2], &[x as f32, y as f32]);

        let hot = Spark::new(
            chemistry::CHARCOAL_TAIL.sparks.as_ref().unwrap(),
            Vec3::new(0.0, 120.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
        );
        trails.clear();
        render_spark(&hot, &camera, &mut points, &mut trails);
        assert_eq!(trails.len(), 10);
        assert!(trails[0] > trails[2], "streak trails behind the motion");
        assert!(trails[6] < trails[5], "charcoal sparks glow orange");
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
        assert!(engine.program.launches[..launched]
            .iter()
            .all(|l| l.time <= engine.time));
        engine.resize(0, 0);
        engine.tick(Clock::STEP_SECONDS);
        assert_eq!(engine.cols(), 0);
        assert_eq!(engine.rows(), 0);
    }
}
