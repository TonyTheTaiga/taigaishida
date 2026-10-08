//! Show programmes: what fires, from where, at what angle, and when.
//!
//! Display designers script a show by when each effect should be *seen*:
//! shells are cued by burst time and fired early by their predicted flight,
//! water shells by when they land, and comets, mines, cakes, candles, and
//! gerbs by when they fire. Every cue fires from a mortar on a barge deck.
//!
//! Modern displays keep every layer of the sky busy (Macy's fires about
//! 2,000 effects a minute; finales fire a fifth of a show in its last
//! minute or so). One programme (`programme.rs`), written in the cue-sheet
//! vocabulary of `sheet.rs`, plays at every venue: five barges on desktop,
//! three on a portrait phone with half the particle budget.

mod programme;
mod sheet;

use crate::comet::{Comet, Gerb, Mine};
#[cfg(test)]
use crate::designs::*;
#[cfg(test)]
use crate::fleet::LENGTH;
use crate::fleet::{Barge, LAMPS_PER_BARGE};
use crate::particle::Vec3;
use crate::projection::Stage;
use crate::rng::Rng;
use crate::shell::{Fuse, Ignition, Shell, ShellDesign};
use crate::star::{Puff, Star};
use crate::world::Spawn;
use programme::Plan;
use sheet::Sheet;

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
    pub lamps: usize,
}

impl Budget {
    /// Stars draw four history segments and sparks one.
    pub const fn segments(&self) -> usize {
        self.stars * 4 + self.sparks
    }
}

/// Five barges 110 m apart: a 480 m firing line.
const DESKTOP_FLEET: [Barge; 5] = [
    Barge {
        x: -220.0,
        z: 0.0,
        tug: -1.0,
    },
    Barge {
        x: -110.0,
        z: 0.0,
        tug: 1.0,
    },
    Barge {
        x: 0.0,
        z: 0.0,
        tug: -1.0,
    },
    Barge {
        x: 110.0,
        z: 0.0,
        tug: -1.0,
    },
    Barge {
        x: 220.0,
        z: 0.0,
        tug: 1.0,
    },
];

/// Three barges close together, framed by a portrait screen.
const MOBILE_FLEET: [Barge; 3] = [
    Barge {
        x: -55.0,
        z: 0.0,
        tug: -1.0,
    },
    Barge {
        x: 0.0,
        z: 0.0,
        tug: 1.0,
    },
    Barge {
        x: 55.0,
        z: 0.0,
        tug: 1.0,
    },
];

impl Venue {
    pub fn stage(self) -> Stage {
        match self {
            Venue::Desktop => Stage::DESKTOP,
            Venue::Mobile => Stage::MOBILE,
        }
    }

    pub fn fleet(self) -> &'static [Barge] {
        match self {
            Venue::Desktop => &DESKTOP_FLEET,
            Venue::Mobile => &MOBILE_FLEET,
        }
    }

    /// Phones get half the particles: their GPUs fill far fewer pixels per
    /// frame, and the smaller screen shows less detail anyway.
    pub const fn budget(self) -> Budget {
        match self {
            Venue::Desktop => Budget {
                stars: 10000,
                sparks: 36000,
                puffs: 2000,
                lamps: DESKTOP_FLEET.len() * LAMPS_PER_BARGE,
            },
            Venue::Mobile => Budget {
                stars: 4500,
                sparks: 15000,
                puffs: 900,
                lamps: MOBILE_FLEET.len() * LAMPS_PER_BARGE,
            },
        }
    }

    fn plan(self) -> &'static Plan {
        match self {
            Venue::Desktop => &programme::DESKTOP,
            Venue::Mobile => &programme::MOBILE,
        }
    }

    /// The programme for one loop, its random draws taken from `rng`.
    pub fn program(self, rng: &mut Rng) -> Program {
        let mut sheet = Sheet::new(self, rng);
        let duration = programme::play(&mut sheet, self.plan());
        sheet.finish(duration)
    }
}

#[derive(Clone, Copy)]
pub enum Device {
    Shell(&'static ShellDesign),
    Comet(&'static Comet),
    Mine(&'static Mine),
    Gerb(&'static Gerb),
}

impl Device {
    /// The lift charge, or `None` for a gerb, which has none.
    fn lift_charge(self) -> Option<f64> {
        match self {
            Device::Shell(design) => Some(design.lift_charge),
            Device::Comet(comet) => Some(comet.lift_charge),
            Device::Mine(mine) => Some(mine.lift_charge),
            Device::Gerb(_) => None,
        }
    }

    #[cfg(test)]
    pub fn name(self) -> &'static str {
        match self {
            Device::Shell(design) => design.name,
            Device::Comet(comet) => comet.name,
            Device::Mine(mine) => mine.name,
            Device::Gerb(gerb) => gerb.name,
        }
    }
}

pub struct Cue {
    /// Seconds into the show when the lift charge fires.
    pub time: f64,
    /// When the audience sees it: a shell's burst, a water shell's landing,
    /// or the moment a comet, mine, or gerb fires.
    #[cfg_attr(not(test), allow(dead_code))]
    pub show: f64,
    pub device: Device,
    /// The tube's mouth on a barge deck.
    pub mortar: Vec3,
    /// Tube tilt from vertical toward +x, radians.
    pub tilt: f64,
    /// Time-fused shells only: the fuse, cut to the shell's nominal time to
    /// apex.
    pub fuse: f64,
}

impl Cue {
    /// A shell timed to burst at `burst` from a mortar tilted `tilt_degrees`
    /// toward +x. Display designers script by burst time and fire early by
    /// the shell's predicted flight.
    pub fn shell(
        burst: f64,
        design: &'static ShellDesign,
        mortar: Vec3,
        tilt_degrees: f64,
    ) -> Self {
        debug_assert!(design.ignition == Ignition::TimeFuse);
        let tilt = tilt_degrees.to_radians();
        let (_, fuse) = design.flight(tilt);
        let mean_fuse = fuse * 0.5 * (FUSE_SCATTER.0 + FUSE_SCATTER.1);
        Self {
            time: burst - mean_fuse,
            show: burst,
            device: Device::Shell(design),
            mortar,
            tilt,
            fuse,
        }
    }

    /// A water shell lobbed to land, and burst, at `landing`.
    pub fn water(
        landing: f64,
        design: &'static ShellDesign,
        mortar: Vec3,
        tilt_degrees: f64,
    ) -> Self {
        debug_assert!(design.ignition == Ignition::Contact);
        let tilt = tilt_degrees.to_radians();
        let (_, flight) = design.splashdown(tilt);
        Self {
            time: landing - flight,
            show: landing,
            device: Device::Shell(design),
            mortar,
            tilt,
            fuse: 0.0,
        }
    }

    /// A comet, mine, or gerb, which shows the moment it fires or lights.
    pub fn ground(time: f64, device: Device, mortar: Vec3, tilt_degrees: f64) -> Self {
        Self {
            time,
            show: time,
            device,
            mortar,
            tilt: tilt_degrees.to_radians(),
            fuse: 0.0,
        }
    }

    /// Fire the device. Lift charges and tube placement vary a little from
    /// shot to shot, and every lift leaves a flash and a puff of smoke.
    pub fn fire(&self, spawn: &mut Spawn) {
        let rng = &mut *spawn.rng;
        let tilt = self.tilt + rng.range(-1.0, 1.0).to_radians();
        let lean = rng.range(-1.5, 1.5).to_radians();
        let aim = Vec3::new(tilt.sin(), tilt.cos() * lean.cos(), tilt.cos() * lean.sin());
        match self.device {
            Device::Shell(design) => {
                let speed = design.muzzle_speed() * rng.range(0.97, 1.03);
                let fuse = match design.ignition {
                    Ignition::TimeFuse => {
                        Fuse::Burning(self.fuse * rng.range(FUSE_SCATTER.0, FUSE_SCATTER.1))
                    }
                    Ignition::Contact => Fuse::Contact,
                };
                let shell = Shell::new(design, self.mortar, aim.scale(speed), fuse, rng);
                spawn.shells.push(shell);
            }
            Device::Comet(comet) => {
                let speed = comet.muzzle_speed() * rng.range(0.97, 1.03);
                let star = Star::new(comet.recipe, 1.0, self.mortar, aim.scale(speed), rng);
                spawn.stars.push(star.varied(rng));
            }
            Device::Gerb(gerb) => {
                let star = Star::new(gerb.recipe, 1.0, self.mortar, Vec3::default(), rng);
                spawn.stars.push(star.mounted());
            }
            Device::Mine(mine) => {
                let speed = mine.muzzle_speed();
                let (u, v) = aim.basis();
                for &(recipe, count) in mine.payload {
                    for _ in 0..count {
                        // Uniform over the cone's solid angle.
                        let cos = rng.range(mine.spread.cos(), 1.0);
                        let sin = (1.0 - cos * cos).sqrt();
                        let turn = rng.range(0.0, std::f64::consts::TAU);
                        let direction = aim
                            .scale(cos)
                            .add(u.scale(sin * turn.cos()))
                            .add(v.scale(sin * turn.sin()));
                        let velocity = direction.scale(speed * rng.range(0.8, 1.05));
                        let star = Star::new(recipe, 1.0, self.mortar, velocity, rng)
                            .varied(rng)
                            .primed(0.05, rng);
                        spawn.stars.push(star);
                    }
                }
            }
        }
        // A lift's flash and smoke scale with its charge, relative to a
        // 6-inch bursting charge. A gerb has no lift.
        let Some(lift) = self.device.lift_charge() else {
            return;
        };
        let size = (lift / 0.27).cbrt();
        spawn.puffs.push(Puff::flash(
            self.mortar.add(Vec3::new(0.0, 1.0, 0.0)),
            0.6 * size,
        ));
        spawn.puffs.push(Puff::smoke(
            self.mortar.add(Vec3::new(0.0, 2.0, 0.0)),
            Vec3::new(0.0, 2.5, 0.0).add(rng.unit().scale(0.5)),
            1.0 + 6.0 * size,
            0.4,
            rng.range(10.0, 16.0),
        ));
    }

    /// Where a shell is predicted to burst.
    #[cfg(test)]
    pub fn burst_point(&self) -> Option<Vec3> {
        match self.device {
            Device::Shell(design) if design.ignition == Ignition::Contact => {
                let (landing, _) = design.splashdown(self.tilt);
                Some(self.mortar.add(landing))
            }
            Device::Shell(design) => {
                let (apex, _) = design.flight(self.tilt);
                Some(self.mortar.add(apex))
            }
            _ => None,
        }
    }
}

pub struct Program {
    /// Sorted by firing time.
    pub cues: Vec<Cue>,
    /// Length of one loop, including the dark pause before it restarts.
    pub duration: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::particle::Clock;
    use crate::projection::Camera;
    use crate::world::World;

    fn program(venue: Venue) -> Program {
        venue.program(&mut Rng::new(42))
    }

    const VENUES: [(Venue, &[(f64, f64)]); 2] = [
        // Desktop, laptop, ultrawide, and narrow windows.
        (
            Venue::Desktop,
            &[
                (1918.0, 900.0),
                (1280.0, 800.0),
                (2520.0, 900.0),
                (1024.0, 1026.0),
            ],
        ),
        // Phones in portrait and landscape.
        (
            Venue::Mobile,
            &[(390.0, 844.0), (412.0, 915.0), (844.0, 390.0)],
        ),
    ];

    #[test]
    fn programmes_feature_every_signature_and_modern_shell() {
        for (venue, _) in VENUES {
            let program = program(venue);
            for design in ALL.iter().chain(MODERN) {
                assert!(
                    program
                        .cues
                        .iter()
                        .any(|cue| cue.device.name() == design.name),
                    "{venue:?} lacks {}",
                    design.name
                );
            }
            assert!(program.cues.windows(2).all(|w| w[0].time <= w[1].time));
            assert!(program.cues[0].time >= 0.0, "{venue:?} fires early");
            let last = program.cues.iter().map(|cue| cue.show).fold(0.0, f64::max);
            // The last shells (kamuro) need ~11 s to fade, then a pause.
            assert!(
                program.duration - last >= 12.0,
                "{venue:?} loops {} s after its last burst",
                program.duration - last
            );
        }
    }

    #[test]
    fn the_sky_is_never_empty_and_the_finale_is_densest() {
        for (venue, _) in VENUES {
            let program = program(venue);
            let mut shows: Vec<f64> = program.cues.iter().map(|cue| cue.show).collect();
            shows.sort_by(f64::total_cmp);
            let last = *shows.last().unwrap();
            // Never more than 0.6 s without something new in the sky.
            let gap = shows
                .windows(2)
                .map(|w| (w[1] - w[0], w[0]))
                .fold((0.0, 0.0), |a, b| if b.0 > a.0 { b } else { a });
            assert!(
                gap.0 <= 0.6,
                "{venue:?} goes dark for {:.2} s at {:.1} s",
                gap.0,
                gap.1
            );
            let rate = |from: f64, to: f64| {
                shows.iter().filter(|&&t| t >= from && t < to).count() as f64 / (to - from)
            };
            let body = rate(1.0, last - 40.0);
            let finale = rate(last - 40.0, last + 0.01);
            println!(
                "{venue:?}: {} cues over {:.0} s, {body:.1}/s in the body, {finale:.1}/s in the finale",
                shows.len(),
                program.duration
            );
            assert!(body >= 3.0, "{venue:?} body {body:.1}/s");
            assert!(finale > body, "{venue:?} finale {finale:.1}/s");
        }
    }

    #[test]
    fn every_cue_fires_from_a_barge_deck() {
        for (venue, _) in VENUES {
            for cue in program(venue).cues {
                let on_deck = venue.fleet().iter().any(|barge| {
                    (cue.mortar.x - barge.x).abs() < LENGTH * 0.5
                        && (cue.mortar.z - barge.z).abs() < crate::fleet::BEAM * 0.5
                });
                assert!(
                    on_deck,
                    "{venue:?}: {} fires from the water",
                    cue.device.name()
                );
            }
        }
    }

    #[test]
    fn every_burst_lands_inside_the_frame() {
        for (venue, viewports) in VENUES {
            let program = program(venue);
            for &(width, height) in viewports {
                let camera = Camera::new(width, height, venue.stage());
                for cue in &program.cues {
                    let Some(p) = cue.burst_point() else {
                        continue;
                    };
                    let (x, y, _) = camera.project(p.x, p.y, p.z).unwrap();
                    assert!(
                        (0.0..width).contains(&x) && y > 0.0 && y < height,
                        "{venue:?} {width}x{height}: {} from x={} bursts off screen at ({x:.1}, {y:.1})",
                        cue.device.name(),
                        cue.mortar.x
                    );
                }
            }
        }
    }

    #[test]
    fn shells_burst_on_cue() {
        let budget = Venue::Desktop.budget();
        for cue in program(Venue::Desktop).cues.iter().step_by(7) {
            let mut world = World::new(&budget, 3);
            world.fire(cue);
            if world.shells.is_empty() {
                continue;
            }
            let mut time = cue.time;
            while world.bursts.is_empty() && !world.shells.is_empty() {
                world.step(Clock::STEP_SECONDS);
                time += Clock::STEP_SECONDS;
            }
            assert!(
                (time - cue.show).abs() < 0.3,
                "{} burst at {time:.2} s, cued for {:.2} s",
                cue.device.name(),
                cue.show
            );
        }
    }
}
