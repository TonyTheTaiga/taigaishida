//! Show programmes: what fires, from where, at what angle, and when.
//!
//! Display designers script a show by when each shell should *burst*, then
//! fire it early by its flight time. Cues here work the same way: each names a
//! burst time, a mortar position along the barge, and a tilt, and the launch
//! time is worked back from the shell's predicted flight.
//!
//! There are two programmes. The desktop show is panoramic: mortars span a
//! 450 m barge, salvos fill the width, and star mines sweep side to side. The
//! mobile show is composed for a portrait phone: one centred column of shells,
//! stacked by height instead of spread by width, on half the particle budget.

use crate::designs::*;
use crate::particle::Vec3;
use crate::projection::Stage;
use crate::shell::{Shell, ShellDesign};
use crate::{pick, rand};

/// Time fuses burn up to 2% short, so shells burst at or just before the apex.
const FUSE_SCATTER: (f64, f64) = (0.98, 1.0);

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Venue {
    Desktop,
    Mobile,
}

pub struct Budget {
    pub stars: usize,
    pub sparks: usize,
    pub puffs: usize,
}

impl Venue {
    pub fn stage(self) -> Stage {
        match self {
            Venue::Desktop => Stage::DESKTOP,
            Venue::Mobile => Stage::MOBILE,
        }
    }

    /// Phones get half the particles: their GPUs fill far fewer pixels per
    /// frame, and the smaller screen shows less detail anyway.
    pub const fn budget(self) -> Budget {
        match self {
            Venue::Desktop => Budget {
                stars: 6000,
                sparks: 26000,
                puffs: 800,
            },
            Venue::Mobile => Budget {
                stars: 3000,
                sparks: 12000,
                puffs: 400,
            },
        }
    }

    pub fn program(self) -> Program {
        match self {
            Venue::Desktop => desktop(),
            Venue::Mobile => mobile(),
        }
    }
}

pub struct Launch {
    /// Seconds into the show when the lift charge fires.
    pub time: f64,
    pub design: &'static ShellDesign,
    /// Mortar position along the barge, m.
    pub x: f64,
    /// Mortar tilt from vertical toward +x, radians.
    pub tilt: f64,
    /// Time fuse, cut to the shell's nominal time to apex.
    pub fuse: f64,
}

impl Launch {
    /// A shell timed to burst at `burst` from a mortar at `x` tilted
    /// `tilt_degrees` toward +x.
    pub fn new(burst: f64, design: &'static ShellDesign, x: f64, tilt_degrees: f64) -> Self {
        let tilt = tilt_degrees.to_radians();
        let (_, fuse) = design.flight(tilt);
        let mean_fuse = fuse * 0.5 * (FUSE_SCATTER.0 + FUSE_SCATTER.1);
        Self {
            time: burst - mean_fuse,
            design,
            x,
            tilt,
            fuse,
        }
    }

    /// Fire the shell. Lift charges, mortar placement, and fuses all vary a
    /// little from shot to shot.
    pub fn fire(&self) -> Shell {
        let speed = self.design.muzzle_speed() * rand(0.97, 1.03);
        let tilt = self.tilt + rand(-1.0, 1.0).to_radians();
        Shell::new(
            self.design,
            Vec3::new(self.x, 0.0, rand(-15.0, 15.0)),
            Vec3::new(speed * tilt.sin(), speed * tilt.cos(), 0.0),
            self.fuse * rand(FUSE_SCATTER.0, FUSE_SCATTER.1),
        )
    }

    /// Where the shell is predicted to burst.
    #[cfg(test)]
    pub fn burst_point(&self) -> Vec3 {
        let (apex, _) = self.design.flight(self.tilt);
        Vec3::new(self.x + apex.x, apex.y, 0.0)
    }

    #[cfg(test)]
    pub fn burst_time(&self) -> f64 {
        self.time + self.fuse * 0.5 * (FUSE_SCATTER.0 + FUSE_SCATTER.1)
    }
}

pub struct Program {
    /// Sorted by launch time.
    pub launches: Vec<Launch>,
    /// Length of one loop, including the dark pause before it restarts.
    pub duration: f64,
}

/// A cue sheet: the vocabulary display designers compose with.
#[derive(Default)]
struct Sheet {
    launches: Vec<Launch>,
}

impl Sheet {
    fn shot(&mut self, burst: f64, design: &'static ShellDesign, x: f64, tilt: f64) {
        self.launches.push(Launch::new(burst, design, x, tilt));
    }

    /// Several mortars fired so their shells burst as one.
    fn salvo(&mut self, burst: f64, design: &'static ShellDesign, xs: &[f64]) {
        for &x in xs {
            self.shot(burst, design, x, 0.0);
        }
    }

    /// One rack of mortars splayed evenly between ±`spread` degrees.
    fn fan(
        &mut self,
        burst: f64,
        designs: &[&'static ShellDesign],
        x: f64,
        count: usize,
        spread: f64,
    ) {
        for i in 0..count {
            let tilt = if count > 1 {
                -spread + 2.0 * spread * i as f64 / (count - 1) as f64
            } else {
                0.0
            };
            self.shot(burst, designs[i % designs.len()], x, tilt);
        }
    }

    /// Shells bursting one after another along a line of mortars.
    fn sweep(
        &mut self,
        burst: f64,
        designs: &[&'static ShellDesign],
        from: f64,
        to: f64,
        count: usize,
        interval: f64,
    ) {
        for i in 0..count {
            let fraction = i as f64 / (count.max(2) - 1) as f64;
            let x = from + (to - from) * fraction;
            self.shot(
                burst + i as f64 * interval,
                designs[i % designs.len()],
                x,
                0.0,
            );
        }
    }

    /// Different sizes from one spot, timed to burst together in tiers.
    fn ladder(&mut self, burst: f64, designs: &[&'static ShellDesign], x: f64) {
        for &design in designs {
            self.shot(burst, design, x, 0.0);
        }
    }

    /// A star mine: rapid fire from random mortars, the gap between bursts
    /// shrinking from `start_gap` to `end_gap` seconds.
    #[allow(clippy::too_many_arguments)]
    fn mine(
        &mut self,
        from: f64,
        to: f64,
        designs: &[&'static ShellDesign],
        half_width: f64,
        max_tilt: f64,
        start_gap: f64,
        end_gap: f64,
    ) {
        let mut t = from;
        while t < to {
            self.shot(
                t,
                pick(designs),
                rand(-half_width, half_width),
                rand(-max_tilt, max_tilt),
            );
            let progress = (t - from) / (to - from);
            t += start_gap + (end_gap - start_gap) * progress;
        }
    }

    fn finish(mut self, duration: f64) -> Program {
        self.launches.sort_by(|a, b| a.time.total_cmp(&b.time));
        Program {
            launches: self.launches,
            duration,
        }
    }
}

/// About 2½ minutes across a 450 m barge.
fn desktop() -> Program {
    let mut s = Sheet::default();

    // Opening: a silver salute, then three chrysanthemums across the barge.
    s.fan(5.0, &[&PEONY_SILVER], 0.0, 5, 30.0);
    s.fan(6.0, &[&PEONY_RED], -170.0, 3, 15.0);
    s.fan(6.0, &[&PEONY_BLUE], 170.0, 3, 15.0);
    s.salvo(7.5, &YAEZAKI, &[-150.0, 0.0, 150.0]);
    s.sweep(11.5, &[&GOLD_KIKU], -200.0, 200.0, 7, 0.25);
    s.shot(14.0, &CROSSETTE, -110.0, -8.0);
    s.shot(14.0, &CROSSETTE, 110.0, 8.0);

    // Tanpatsu: the signature shells one at a time, each given room to finish.
    s.shot(19.0, &GHOST, 0.0, 0.0);
    s.shot(25.0, &SATURN, -90.0, 0.0);
    s.shot(30.5, &DRAGON_EGGS, 90.0, 0.0);
    s.shot(36.0, &TWINKLING, 0.0, 0.0);
    s.shot(43.0, &SENRIN, -60.0, 0.0);
    s.shot(48.5, &KOI, 60.0, 0.0);
    s.shot(53.0, &PALM, -130.0, -4.0);
    s.shot(53.3, &PALM, 130.0, 4.0);
    s.shot(61.0, &KAMURO, 0.0, 0.0);

    // Star mine: colour sweeps, a crossette fan, then call and response.
    s.sweep(73.0, PEONIES, -210.0, 210.0, 10, 0.4);
    s.sweep(77.2, PEONIES, 210.0, -210.0, 10, 0.4);
    s.fan(81.5, &[&CROSSETTE], 0.0, 5, 28.0);
    let calls: [(&'static ShellDesign, f64); 5] = [
        (&PEONY_GREEN, -160.0),
        (&PEONY_RED, 160.0),
        (&PEONY_VIOLET, -160.0),
        (&PEONY_BLUE, 160.0),
        (&PEONY_SILVER, 0.0),
    ];
    for (i, (design, x)) in calls.into_iter().enumerate() {
        s.fan(84.0 + i as f64, &[design], x, 3, 14.0);
    }
    s.salvo(89.5, &GOLD_KIKU, &[-200.0, -100.0, 0.0, 100.0, 200.0]);
    s.mine(91.0, 94.5, STAR_MINE, 220.0, 12.0, 0.3, 0.2);

    // Garden: layered heights, small shells low under large ones.
    s.shot(98.0, &SENRIN, 0.0, 0.0);
    s.salvo(98.6, &PALM, &[-170.0, 170.0]);
    s.salvo(104.0, &SATURN, &[-110.0, 110.0]);
    s.ladder(
        109.5,
        &[&PEONY_RED, &GOLD_KIKU, &DRAGON_EGGS, &TWINKLING, &YAEZAKI],
        0.0,
    );
    s.salvo(115.5, &KOI, &[-120.0, 120.0]);
    s.shot(116.0, &DRAGON_EGGS, 0.0, 0.0);

    // Crescendo: bigger salvos over an accelerating star mine.
    s.salvo(120.0, &GHOST, &[-150.0, 0.0, 150.0]);
    s.salvo(123.5, &TWINKLING, &[-90.0, 90.0]);
    s.fan(123.5, &[&PEONY_SILVER], 0.0, 5, 25.0);
    s.sweep(126.5, &[&YAEZAKI], -200.0, 200.0, 5, 0.5);
    s.fan(129.5, &[&CROSSETTE], -120.0, 3, 20.0);
    s.fan(129.5, &[&CROSSETTE], 120.0, 3, 20.0);
    s.mine(130.5, 136.0, STAR_MINE, 230.0, 15.0, 0.3, 0.12);

    // Finale: full-width salvos, then a kamuro curtain that hangs to the end.
    s.salvo(136.5, &YAEZAKI, &[-160.0, 0.0, 160.0]);
    s.salvo(136.5, &GOLD_KIKU, &[-240.0, -80.0, 80.0, 240.0]);
    s.salvo(138.5, &TWINKLING, &[-200.0, -100.0, 0.0, 100.0, 200.0]);
    s.salvo(140.5, &DRAGON_EGGS, &[-200.0, -100.0, 0.0, 100.0, 200.0]);
    s.salvo(
        142.5,
        &KAMURO,
        &[-210.0, -140.0, -70.0, 0.0, 70.0, 140.0, 210.0],
    );
    s.finish(158.0)
}

/// About 1½ minutes in one centred column, for a portrait phone. Each shell
/// follows the last as it fades, so the small screen is never dark for long.
fn mobile() -> Program {
    let mut s = Sheet::default();

    // Opening: a small silver salute, then a chrysanthemum that fills the screen.
    s.fan(5.0, &[&PEONY_SILVER], 0.0, 3, 12.0);
    s.shot(7.0, &YAEZAKI, 0.0, 0.0);

    // Tanpatsu: the signature shells, centred, with small shells layered under
    // the long-lived ones.
    s.shot(11.5, &GHOST, 0.0, 0.0);
    s.shot(16.0, &SATURN, 0.0, 0.0);
    s.shot(21.0, &TWINKLING, 0.0, 0.0);
    s.fan(24.5, &[&PEONY_RED], 0.0, 3, 14.0);
    s.ladder(28.5, &[&PEONY_RED, &GOLD_KIKU, &DRAGON_EGGS], 0.0);
    s.shot(32.5, &KOI, 0.0, 0.0);
    s.shot(36.0, &PALM, 0.0, 0.0);
    s.shot(41.0, &SENRIN, 0.0, 0.0);
    s.shot(45.5, &CROSSETTE, -35.0, -8.0);
    s.shot(45.5, &CROSSETTE, 35.0, 8.0);

    // Star mine: narrow sweeps that stay inside a portrait frame.
    s.sweep(49.0, PEONIES, -55.0, 55.0, 6, 0.45);
    s.sweep(51.8, PEONIES, 55.0, -55.0, 6, 0.45);
    s.salvo(55.0, &GOLD_KIKU, &[-50.0, 0.0, 50.0]);
    s.shot(58.0, &DRAGON_EGGS, 0.0, 0.0);

    // Finale: paired chrysanthemums, twinkles, then a three-shell kamuro.
    s.salvo(62.0, &YAEZAKI, &[-40.0, 40.0]);
    s.shot(65.5, &TWINKLING, 0.0, 0.0);
    s.fan(65.5, &[&PEONY_SILVER], 0.0, 3, 15.0);
    s.mine(67.0, 70.0, PEONIES, 55.0, 10.0, 0.35, 0.25);
    s.salvo(71.5, &KAMURO, &[-50.0, 0.0, 50.0]);
    s.finish(87.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particle::{Clock, Particles};
    use crate::projection::Camera;
    use crate::star::Emission;

    const VENUES: [(Venue, &[(usize, usize)]); 2] = [
        // Desktop, laptop, and ultrawide windows.
        (Venue::Desktop, &[(137, 50), (91, 44), (180, 50), (73, 57)]),
        // Phones in portrait and landscape.
        (Venue::Mobile, &[(27, 46), (29, 51), (60, 21)]),
    ];

    #[test]
    fn programmes_feature_every_signature_shell_and_start_from_rest() {
        for (venue, _) in VENUES {
            let program = venue.program();
            for design in ALL {
                assert!(
                    program
                        .launches
                        .iter()
                        .any(|l| l.design.name == design.name),
                    "{venue:?} lacks {}",
                    design.name
                );
            }
            assert!(program.launches.windows(2).all(|w| w[0].time <= w[1].time));
            assert!(program.launches[0].time >= 0.0, "{venue:?} fires early");
            let last = program
                .launches
                .iter()
                .map(Launch::burst_time)
                .fold(0.0, f64::max);
            // The last shell (a kamuro) needs ~11 s to fade, then a pause.
            assert!(
                program.duration - last >= 12.0,
                "{venue:?} loops {} s after its last burst",
                program.duration - last
            );
            println!(
                "{venue:?}: {} shells, {:.0} s",
                program.launches.len(),
                program.duration
            );
        }
    }

    #[test]
    fn every_burst_lands_inside_the_frame() {
        for (venue, viewports) in VENUES {
            let program = venue.program();
            for &(cols, rows) in viewports {
                let camera = Camera::new(cols, rows, venue.stage());
                for launch in &program.launches {
                    let p = launch.burst_point();
                    let (x, y, _) = camera.project(p.x, p.y, 0.0).unwrap();
                    assert!(
                        (0.0..cols as f64).contains(&x) && y > 0.0 && y < rows as f64 * 0.72,
                        "{venue:?} {cols}x{rows}: {} from x={} bursts off screen at ({x:.1}, {y:.1})",
                        launch.design.name,
                        launch.x
                    );
                }
            }
        }
    }

    #[test]
    fn mobile_stays_in_one_column_and_never_crowds_the_screen() {
        let program = Venue::Mobile.program();
        for launch in &program.launches {
            assert!(
                launch.burst_point().x.abs() <= 75.0,
                "{}",
                launch.design.name
            );
        }
        // At most three large shells burst within any two seconds.
        let large: Vec<f64> = program
            .launches
            .iter()
            .filter(|l| l.design.diameter > 0.12)
            .map(Launch::burst_time)
            .collect();
        for &t in &large {
            let crowd = large.iter().filter(|&&u| (u - t).abs() < 1.0).count();
            assert!(crowd <= 3, "{crowd} large shells around {t:.1} s");
        }
    }

    #[test]
    fn shells_burst_on_cue() {
        let program = Venue::Desktop.program();
        for launch in program.launches.iter().step_by(9) {
            let mut shell = launch.fire();
            let mut sparks = Particles::new(1000);
            let mut stars = Vec::new();
            let mut emission = Emission {
                sparks: &mut sparks,
                stars: &mut stars,
                spark_rate: 0.0,
            };
            let mut time = launch.time;
            while matches!(
                shell.update(Clock::STEP_SECONDS, &mut emission),
                crate::shell::Flight::Climbing
            ) {
                time += Clock::STEP_SECONDS;
            }
            assert!(
                (time - launch.burst_time()).abs() < 0.3,
                "{} burst at {time:.2} s, cued for {:.2} s",
                launch.design.name,
                launch.burst_time()
            );
        }
    }
}
