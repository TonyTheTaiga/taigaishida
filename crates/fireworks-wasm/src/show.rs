//! Show programmes: what fires, from where, at what angle, and when.
//!
//! Display designers script a show by when each effect should be *seen*:
//! shells are cued by burst time and fired early by their predicted flight,
//! water shells by when they land, and comets, mines, cakes, candles, and
//! gerbs by when they fire. Every cue fires from a mortar on a barge deck.
//!
//! Modern displays keep every layer of the sky busy (Macy's fires about
//! 2,000 effects a minute; finales fire a fifth of a show in its last
//! minute or so). Both programmes here layer low ground effects, mid-height
//! star-mine chases, and high feature shells so the sky is never empty, and
//! end in their densest minute. The desktop show spans five barges; the
//! mobile show is composed for a portrait phone on three, on half the
//! particle budget.

use crate::comet::{Comet, Gerb, Mine};
use crate::designs::*;
use crate::fleet::{Barge, LAMPS_PER_BARGE, LENGTH};
use crate::particle::Vec3;
use crate::projection::Stage;
use crate::rng::Rng;
use crate::shell::{Fuse, Ignition, Shell, ShellDesign};
use crate::star::{Puff, Star};
use crate::world::Spawn;

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

    /// The programme for one loop, its random draws taken from `rng`.
    pub fn program(self, rng: &mut Rng) -> Program {
        match self {
            Venue::Desktop => desktop(rng),
            Venue::Mobile => mobile(rng),
        }
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

/// A cue sheet: the vocabulary display designers compose with.
struct Sheet<'a> {
    fleet: &'static [Barge],
    cues: Vec<Cue>,
    rng: &'a mut Rng,
}

impl<'a> Sheet<'a> {
    fn new(venue: Venue, rng: &'a mut Rng) -> Self {
        Self {
            fleet: venue.fleet(),
            cues: Vec::new(),
            rng,
        }
    }

    /// Outer racks lean toward the centre of the line, up to 12° at the
    /// ends of the outermost decks.
    fn lean(&self, x: f64) -> f64 {
        let span = self.fleet.iter().map(|b| b.x.abs()).fold(0.0, f64::max) + LENGTH * 0.5;
        (-12.0 * x / span).clamp(-12.0, 12.0)
    }

    /// The deck position nearest to `x` along the firing line.
    fn at(&mut self, x: f64) -> Vec3 {
        let barge = self
            .fleet
            .iter()
            .min_by(|a, b| (a.x - x).abs().total_cmp(&(b.x - x).abs()))
            .expect("a venue has barges");
        let along = (x - barge.x) / (LENGTH * 0.5 - 2.5);
        barge.mortar(along, self.rng.range(-1.0, 1.0))
    }

    fn shot(&mut self, burst: f64, design: &'static ShellDesign, x: f64, tilt: f64) {
        let mortar = self.at(x);
        self.cues.push(Cue::shell(burst, design, mortar, tilt));
    }

    /// Several mortars fired so their shells burst as one.
    fn salvo(&mut self, burst: f64, design: &'static ShellDesign, xs: &[f64]) {
        for &x in xs {
            self.shot(burst, design, x, 0.0);
        }
    }

    /// One shell from every barge.
    fn line(&mut self, burst: f64, design: &'static ShellDesign) {
        for i in 0..self.fleet.len() {
            self.shot(burst, design, self.fleet[i].x, 0.0);
        }
    }

    /// One rack of mortars splayed evenly between ±`spread` degrees, leaned
    /// toward the centre of the line as crews angle their outer racks.
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
            self.shot(burst, designs[i % designs.len()], x, self.lean(x) + tilt);
        }
    }

    /// Shells bursting one after another along the line: a star-mine chase.
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

    /// A star mine: rapid fire from random mortars, the gap between bursts
    /// shrinking from `start_gap` to `end_gap` seconds.
    #[allow(clippy::too_many_arguments)]
    fn star_mine(
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
            let x = self.rng.range(-half_width, half_width);
            let tilt = self.lean(x) + self.rng.range(-max_tilt, max_tilt);
            let design = self.rng.pick(designs);
            self.shot(t, design, x, tilt);
            let progress = (t - from) / (to - from);
            t += start_gap + (end_gap - start_gap) * progress;
        }
    }

    fn ground(&mut self, time: f64, device: Device, x: f64, tilt: f64) {
        let mortar = self.at(x);
        self.cues.push(Cue::ground(time, device, mortar, tilt));
    }

    /// Single tubes fired one after another along the line: a pixel front.
    #[allow(clippy::too_many_arguments)]
    fn chase(
        &mut self,
        time: f64,
        device: Device,
        from: f64,
        to: f64,
        count: usize,
        interval: f64,
        tilt: f64,
    ) {
        for i in 0..count {
            let x = from + (to - from) * i as f64 / (count.max(2) - 1) as f64;
            self.ground(time + i as f64 * interval, device, x, tilt);
        }
    }

    /// A W fan cake: a row of tubes splayed ±`spread` degrees, fired at once.
    fn fan_cake(&mut self, time: f64, device: Device, x: f64, count: usize, spread: f64) {
        let mortar = self.at(x);
        for i in 0..count {
            let tilt = -spread + 2.0 * spread * i as f64 / (count.max(2) - 1) as f64;
            let tilt = self.lean(x) + tilt;
            self.cues.push(Cue::ground(time, device, mortar, tilt));
        }
    }

    /// A Z cake: one tube at a time, snaking between ±`swing` degrees.
    fn z_cake(
        &mut self,
        time: f64,
        device: Device,
        x: f64,
        shots: usize,
        interval: f64,
        swing: f64,
    ) {
        let mortar = self.at(x);
        for i in 0..shots {
            let phase = (i as f64 / 8.0).fract();
            let tilt = swing * (1.0 - 4.0 * (phase - 0.5).abs());
            self.cues.push(Cue::ground(
                time + i as f64 * interval,
                device,
                mortar,
                tilt,
            ));
        }
    }

    /// A Roman candle: pearls one after another from one tube.
    fn candle(
        &mut self,
        time: f64,
        pearls: &[&'static Comet],
        x: f64,
        shots: usize,
        interval: f64,
        tilt: f64,
    ) {
        let mortar = self.at(x);
        for i in 0..shots {
            self.cues.push(Cue::ground(
                time + i as f64 * interval,
                Device::Comet(pearls[i % pearls.len()]),
                mortar,
                tilt,
            ));
        }
    }

    /// Gerbs along every deck.
    fn gerbs(&mut self, time: f64, gerb: &'static Gerb, per_barge: usize) {
        for barge in self.fleet {
            for i in 0..per_barge {
                let along = -0.7 + 1.4 * (i as f64 + 0.5) / per_barge as f64;
                self.cues.push(Cue::ground(
                    time + self.rng.range(0.0, 0.15),
                    Device::Gerb(gerb),
                    barge.mortar(along, -1.0),
                    0.0,
                ));
            }
        }
    }

    /// Water shells lobbed off alternate sides of the barges in turn, each
    /// landing `interval` seconds after the last.
    fn water_fans(&mut self, from: f64, count: usize, interval: f64) {
        for i in 0..count {
            let index = (i * 2 + i / self.fleet.len()) % self.fleet.len();
            let barge = &self.fleet[index];
            // The end barges lob inward only, keeping the fans over the show.
            let side = if index == 0 {
                1.0
            } else if index == self.fleet.len() - 1 {
                -1.0
            } else if i % 2 == 0 {
                1.0
            } else {
                -1.0
            };
            let mortar = barge.mortar(side, -1.0);
            self.cues.push(Cue::water(
                from + i as f64 * interval,
                &WATER_FAN,
                mortar,
                side * self.rng.range(38.0, 50.0),
            ));
        }
    }

    /// A steady bed of low effects from random decks, about `rate` a second,
    /// so the lowest layer of the sky is never empty.
    fn bed(&mut self, from: f64, to: f64, devices: &[Device], rate: f64) {
        let mut t = from;
        while t < to {
            let barge = &self.fleet
                [(self.rng.range(0.0, self.fleet.len() as f64) as usize).min(self.fleet.len() - 1)];
            let mortar = barge.mortar(self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0));
            self.cues.push(Cue::ground(
                t,
                self.rng.pick(devices),
                mortar,
                self.rng.range(-15.0, 15.0),
            ));
            t += (self.rng.range(0.5, 1.5) / rate).min(0.55);
        }
    }

    /// Sort the sheet, and start the loop so its first lift fires at 0.2 s
    /// even when the opening shells are cued to burst within their own
    /// flight time.
    fn finish(mut self, duration: f64) -> Program {
        self.cues.sort_by(|a, b| a.time.total_cmp(&b.time));
        let offset = (0.2 - self.cues.first().map_or(0.0, |cue| cue.time)).max(0.0);
        for cue in &mut self.cues {
            cue.time += offset;
            cue.show += offset;
        }
        Program {
            cues: self.cues,
            duration: duration + offset,
        }
    }
}

const PISTILS: [&ShellDesign; 4] = [&PISTIL_RED, &PISTIL_PINK, &PISTIL_PURPLE, &PISTIL_AQUA];
const MIDS: [&ShellDesign; 6] = [
    &GOLD_KIKU,
    &PISTIL_AQUA,
    &DOUBLE_RING,
    &PISTIL_PINK,
    &GOLD_KIKU,
    &PISTIL_PURPLE,
];

/// About three minutes from five barges 110 m apart. Every layer of the sky
/// stays busy: pearls, comets, mines, gerbs, and water shells low; star-mine
/// chases and pistils in the middle; a feature shell high every few seconds.
/// The finale fires a fifth of the show in its last forty seconds.
fn desktop(rng: &mut Rng) -> Program {
    let mut s = Sheet::new(Venue::Desktop, rng);
    let line = [-220.0, -110.0, 0.0, 110.0, 220.0];
    let silver = Device::Comet(&SILVER_COMET);
    let gold = Device::Comet(&GOLD_COMET);
    let crossette = Device::Comet(&CROSSETTE_COMET);
    let crackle = Device::Comet(&CRACKLE_COMET);
    let strobe = Device::Comet(&STROBE_COMET);
    let strobe_mine = Device::Mine(&STROBE_MINE);
    let pearls_red = Device::Comet(&PEARL_RED);
    let pearls_aqua = Device::Comet(&PEARL_AQUA);

    // Overture: gerbs light the decks, comets race the line both ways, and
    // the first crowns open low, middle, and high at once.
    // Under everything, a low layer that never stops.
    s.bed(
        0.0,
        162.0,
        &[pearls_red, pearls_aqua, Device::Mine(&COLOUR_MINE), crackle],
        1.2,
    );
    s.gerbs(0.8, &SILVER_GERB, 2);
    s.chase(1.2, silver, -235.0, 235.0, 20, 0.05, 0.0);
    s.chase(2.4, crackle, 235.0, -235.0, 20, 0.05, 0.0);
    for &x in &line {
        s.fan_cake(3.4, silver, x, 5, 30.0);
    }
    s.salvo(4.6, &PISTIL_AQUA, &line);
    s.salvo(5.3, &YONDAN, &[-110.0, 110.0]);
    s.z_cake(5.8, gold, -220.0, 40, 0.18, 35.0);
    s.z_cake(5.89, gold, 220.0, 40, 0.18, -35.0);
    s.star_mine(6.0, 13.5, RAINBOW, 120.0, 12.0, 0.3, 0.18);
    s.fan(7.5, &[&HEART], 0.0, 3, 10.0);
    s.salvo(9.0, &BROCADE_TIPS, &[-165.0, 0.0, 165.0]);
    s.salvo(10.5, &DOUBLE_RING, &[-55.0, 55.0]);
    s.line(12.0, &SALUTE);
    for &x in &line {
        s.ground(12.0, strobe_mine, x, 0.0);
    }
    s.shot(13.2, &JIKANSA, 0.0, 0.0);

    // Colour waves: rainbow chases sweep the line in alternate directions,
    // pistils answer, a feature opens high every four seconds, and pearls
    // and mines keep coming below.
    for k in 0..7 {
        let t = 14.0 + 4.0 * k as f64;
        let (from, to) = if k % 2 == 0 {
            (-235.0, 235.0)
        } else {
            (235.0, -235.0)
        };
        s.sweep(t, RAINBOW, from, to, 15, 0.1);
        s.fan(t + 2.0, &[PISTILS[k % 4]], line[(k * 2) % 5], 3, 15.0);
    }
    s.shot(16.0, &SLIDE, 0.0, 0.0);
    s.salvo(20.0, &JIKANSA, &[-110.0, 110.0]);
    s.shot(24.0, &YONDAN, 0.0, 0.0);
    s.fan(28.0, &[&BUTTERFLY], 0.0, 3, 12.0);
    s.salvo(32.0, &SATURN, &[-165.0, 165.0]);
    s.shot(36.0, &GHOST, 0.0, 0.0);
    s.fan(40.0, &[&STAR], -110.0, 2, 10.0);
    s.fan(40.0, &[&SMILEY], 110.0, 2, 10.0);
    for k in 0..15 {
        let x = if k % 2 == 0 { -110.0 } else { 110.0 };
        s.candle(14.0 + 2.0 * k as f64, PEARLS, x, 6, 0.33, -x / 14.0);
    }
    s.bed(
        14.0,
        44.0,
        &[Device::Mine(&COLOUR_MINE), crackle, silver],
        1.4,
    );

    // Showcase: the signature shells high, one every two or three seconds,
    // over a steady pulse of 4-gō shells, water fans, and mines.
    s.shot(44.5, &KAMURO, 0.0, 0.0);
    s.salvo(47.0, &TWINKLING, &[-110.0, 110.0]);
    s.shot(50.0, &SENRIN, 0.0, -6.0);
    s.salvo(52.5, &KOI, &[-165.0, 165.0]);
    s.shot(55.0, &DRAGON_EGGS, 0.0, 0.0);
    s.shot(58.0, &TIME_RAIN_SHELL, 0.0, 0.0);
    s.salvo(61.0, &LEAVES, &[-110.0, 110.0]);
    s.salvo(63.5, &BEES, &[-55.0, 55.0]);
    s.shot(65.0, &PALM, -220.0, 6.0);
    s.shot(65.0, &PALM, 220.0, -6.0);
    s.fan(67.0, &[&CROSSETTE], 0.0, 5, 28.0);
    s.fan(69.0, &[&SPIDER], 0.0, 3, 15.0);
    s.salvo(71.5, &HORSETAIL, &[-165.0, 165.0]);
    s.salvo(74.0, &YAEZAKI, &[-165.0, 0.0, 165.0]);
    s.salvo(76.5, &GHOST, &[-110.0, 110.0]);
    let mut t = 44.0;
    let mut k = 0;
    while t < 78.0 {
        let tilt = s.rng.range(-8.0, 8.0);
        s.shot(t, MIDS[k % MIDS.len()], line[(k * 3) % 5], tilt);
        t += 0.9;
        k += 1;
    }
    s.water_fans(44.0, 20, 0.6);
    s.bed(
        56.0,
        78.0,
        &[
            Device::Mine(&SILVER_MINE),
            Device::Mine(&CRACKLE_MINE),
            gold,
        ],
        1.2,
    );
    s.gerbs(60.0, &GOLD_GERB, 2);

    // Pulse: fan cakes hop between barges every 1.6 s over snaking Z cakes,
    // V chases open from the centre, multi-break colpi climb in steps, and an
    // accelerating star mine ends on a wall of salutes.
    let hops = [0, 4, 1, 3, 2];
    for k in 0..20 {
        let device = if k % 2 == 0 { crossette } else { silver };
        s.fan_cake(78.0 + 1.6 * k as f64, device, line[hops[k % 5]], 7, 35.0);
    }
    for start in [78.0, 92.0] {
        s.z_cake(start, gold, -110.0, 90, 0.15, 30.0);
        s.z_cake(start + 0.07, gold, 110.0, 90, 0.15, -30.0);
    }
    for k in 0..6 {
        let t = 80.0 + 4.0 * k as f64;
        s.sweep(t, RAINBOW, 0.0, 235.0, 8, 0.12);
        s.sweep(t, RAINBOW, 0.0, -235.0, 8, 0.12);
    }
    for (k, &x) in [-110.0, 110.0, -220.0, 220.0, 0.0].iter().enumerate() {
        s.shot(82.0 + 4.0 * k as f64, &COLPI, x, 0.0);
    }
    s.fan(84.0, &[&SPIDER], 0.0, 3, 18.0);
    s.salvo(88.0, &BROCADE_TIPS, &[-165.0, 165.0]);
    s.shot(92.0, &YONDAN, 0.0, 0.0);
    s.line(96.0, &SALUTE);
    s.star_mine(98.0, 108.0, STAR_MINE, 235.0, 15.0, 0.25, 0.1);
    s.bed(98.0, 108.0, &[strobe_mine, strobe], 2.0);
    s.line(108.5, &SALUTE);
    for &x in &line {
        s.ground(108.5, strobe_mine, x, 0.0);
    }

    // Gold and silver: hanging crowns, time rain, and leaves over water
    // fans, willow mines, and gerbs.
    s.salvo(111.0, &KAMURO, &[-165.0, 165.0]);
    s.shot(112.0, &HORSETAIL, 0.0, 0.0);
    s.salvo(115.0, &TIME_RAIN_SHELL, &[-110.0, 110.0]);
    s.shot(117.0, &LEAVES, 0.0, 0.0);
    s.salvo(119.0, &BROCADE_TIPS, &[-220.0, 0.0, 220.0]);
    s.fan(121.0, &[&TWINKLING], 0.0, 3, 15.0);
    s.shot(123.5, &SATURN, 0.0, 0.0);
    s.water_fans(111.0, 18, 0.65);
    s.gerbs(113.0, &SILVER_GERB, 2);
    s.bed(
        108.8,
        126.0,
        &[
            Device::Mine(&WILLOW_MINE),
            gold,
            Device::Comet(&PEARL_LEMON),
        ],
        1.4,
    );

    // Finale. Build: chases every two seconds with pistils, comet walls,
    // and the modern shells in pairs.
    for k in 0..7 {
        let t = 126.0 + 2.0 * k as f64;
        let (from, to) = if k % 2 == 0 {
            (-235.0, 235.0)
        } else {
            (235.0, -235.0)
        };
        s.sweep(t, RAINBOW, from, to, 15, 0.08);
        s.salvo(t + 1.0, PISTILS[k % 4], &[-165.0, 165.0]);
        let comet = if k % 2 == 0 { silver } else { crackle };
        for &x in &line {
            s.fan_cake(t + 0.5, comet, x, 3, 15.0);
        }
    }
    s.salvo(127.5, &YONDAN, &[-110.0, 110.0]);
    s.shot(130.0, &JIKANSA, 0.0, 0.0);
    s.salvo(132.5, &SLIDE, &[-165.0, 165.0]);
    s.salvo(135.0, &YONDAN, &[-220.0, 0.0, 220.0]);
    s.fan(137.5, &[&HEART], 0.0, 3, 12.0);
    s.bed(
        126.0,
        140.0,
        &[
            Device::Mine(&COLOUR_MINE),
            Device::Mine(&CRACKLE_MINE),
            strobe,
        ],
        2.5,
    );
    // Barrage: the densest star mine, crossette fans from every barge, and
    // chrysanthemums across the line.
    s.star_mine(140.0, 150.0, RAINBOW, 235.0, 18.0, 0.12, 0.06);
    for t in [141.0, 144.0, 147.0] {
        for &x in &line {
            s.fan(t, &[&CROSSETTE], x, 3, 16.0);
        }
    }
    s.salvo(142.0, &YAEZAKI, &[-165.0, 0.0, 165.0]);
    s.salvo(146.0, &YAEZAKI, &[-220.0, -55.0, 55.0, 220.0]);
    s.bed(140.0, 150.0, &[strobe_mine, silver], 3.0);
    s.sweep(148.0, &[&SALUTE], -235.0, 235.0, 9, 0.1);
    s.sweep(149.2, &[&SALUTE], 235.0, -235.0, 9, 0.1);
    // Golden curtain.
    s.line(151.0, &KAMURO);
    for &x in &line {
        s.fan_cake(151.5, gold, x, 7, 30.0);
    }
    s.salvo(153.5, &TIME_RAIN_SHELL, &[-165.0, 0.0, 165.0]);
    s.line(156.0, &BROCADE_TIPS);
    s.bed(151.0, 158.0, &[Device::Mine(&WILLOW_MINE), gold], 2.0);
    // Closer: salute chases, a strobe wall, and kamuro left hanging.
    s.sweep(158.5, &[&SALUTE], -235.0, 235.0, 10, 0.08);
    s.sweep(159.4, &[&SALUTE], 235.0, -235.0, 10, 0.08);
    for &x in &line {
        s.ground(159.0, strobe_mine, x, -10.0);
        s.ground(159.0, strobe_mine, x, 10.0);
    }
    s.salvo(161.0, &YONDAN, &[-110.0, 0.0, 110.0]);
    s.line(161.5, &KAMURO);
    s.finish(178.0)
}

/// About a minute and a half from three barges, composed for a portrait
/// phone: the same layers and styles, centred and stacked by height.
fn mobile(rng: &mut Rng) -> Program {
    let mut s = Sheet::new(Venue::Mobile, rng);
    let line = [-55.0, 0.0, 55.0];
    let silver = Device::Comet(&SILVER_COMET);
    let gold = Device::Comet(&GOLD_COMET);
    let crossette = Device::Comet(&CROSSETTE_COMET);
    let crackle = Device::Comet(&CRACKLE_COMET);
    let strobe_mine = Device::Mine(&STROBE_MINE);
    let pearls_red = Device::Comet(&PEARL_RED);
    let pearls_aqua = Device::Comet(&PEARL_AQUA);

    // Overture.
    // Under everything, a low layer that never stops.
    s.bed(
        0.0,
        90.5,
        &[pearls_red, pearls_aqua, Device::Mine(&COLOUR_MINE), crackle],
        1.0,
    );
    s.gerbs(0.8, &SILVER_GERB, 2);
    s.chase(1.2, silver, -70.0, 70.0, 9, 0.07, 0.0);
    s.chase(2.0, crackle, 70.0, -70.0, 9, 0.07, 0.0);
    for &x in &line {
        s.fan_cake(3.0, silver, x, 5, 22.0);
    }
    s.salvo(4.4, &PISTIL_AQUA, &line);
    s.shot(5.2, &YONDAN, 0.0, 0.0);
    s.z_cake(5.5, gold, -55.0, 30, 0.2, 25.0);
    s.z_cake(5.6, gold, 55.0, 30, 0.2, -25.0);
    s.star_mine(6.0, 12.0, RAINBOW, 60.0, 10.0, 0.35, 0.22);
    s.fan(8.0, &[&HEART], 0.0, 3, 8.0);
    s.shot(10.0, &BROCADE_TIPS, 0.0, 0.0);
    s.line(11.5, &SALUTE);

    // Colour waves.
    for k in 0..5 {
        let t = 13.0 + 4.0 * k as f64;
        let (from, to) = if k % 2 == 0 {
            (-70.0, 70.0)
        } else {
            (70.0, -70.0)
        };
        s.sweep(t, RAINBOW, from, to, 7, 0.14);
        s.shot(t + 2.0, PISTILS[k % 4], line[k % 3], 0.0);
    }
    s.shot(15.0, &SLIDE, 0.0, 0.0);
    s.shot(19.0, &JIKANSA, 0.0, 0.0);
    s.fan(23.0, &[&BUTTERFLY], 0.0, 3, 10.0);
    s.shot(27.0, &GHOST, 0.0, 0.0);
    s.fan(30.5, &[&STAR], 0.0, 2, 8.0);
    s.fan(31.5, &[&SMILEY], 0.0, 2, 8.0);
    for k in 0..10 {
        let x = line[k % 3];
        s.candle(13.0 + 2.0 * k as f64, PEARLS, x, 5, 0.35, -x / 8.0);
    }
    s.bed(13.0, 33.0, &[Device::Mine(&COLOUR_MINE), crackle], 0.8);

    // Showcase.
    s.shot(33.5, &KAMURO, 0.0, 0.0);
    s.shot(36.0, &TWINKLING, 0.0, 0.0);
    s.shot(38.5, &SENRIN, 0.0, 0.0);
    s.shot(41.0, &KOI, 0.0, 0.0);
    s.shot(43.0, &DRAGON_EGGS, 0.0, 0.0);
    s.shot(45.5, &TIME_RAIN_SHELL, 0.0, 0.0);
    s.shot(48.0, &LEAVES, 0.0, 0.0);
    s.shot(50.0, &BEES, 0.0, 0.0);
    s.shot(51.5, &PALM, 0.0, 0.0);
    s.fan(53.0, &[&CROSSETTE], 0.0, 3, 15.0);
    s.shot(54.5, &SPIDER, 0.0, 0.0);
    s.shot(56.0, &HORSETAIL, 0.0, 0.0);
    s.shot(57.5, &SATURN, 0.0, 0.0);
    let mut t = 33.0;
    let mut k = 0;
    while t < 58.0 {
        let x = line[[0, 2][k % 2]];
        let tilt = -x / 12.0 + s.rng.range(-4.0, 4.0);
        s.shot(t, MIDS[k % MIDS.len()], x, tilt);
        t += 1.2;
        k += 1;
    }
    s.water_fans(33.0, 14, 0.7);
    s.bed(
        43.0,
        58.0,
        &[Device::Mine(&SILVER_MINE), Device::Mine(&CRACKLE_MINE)],
        0.9,
    );
    s.gerbs(46.0, &GOLD_GERB, 2);

    // Pulse.
    for k in 0..10 {
        let device = if k % 2 == 0 { crossette } else { silver };
        s.fan_cake(58.0 + 1.6 * k as f64, device, line[k % 3], 5, 28.0);
    }
    s.shot(60.0, &COLPI, 0.0, 0.0);
    s.shot(64.0, &COLPI, -55.0, 0.0);
    s.shot(66.0, &YONDAN, 0.0, 0.0);
    s.shot(68.0, &BROCADE_TIPS, 0.0, 0.0);
    s.star_mine(66.0, 73.0, STAR_MINE, 70.0, 12.0, 0.25, 0.12);
    s.line(73.5, &SALUTE);

    // Finale.
    for k in 0..4 {
        let t = 75.0 + 2.0 * k as f64;
        let (from, to) = if k % 2 == 0 {
            (-70.0, 70.0)
        } else {
            (70.0, -70.0)
        };
        s.sweep(t, RAINBOW, from, to, 7, 0.1);
        s.shot(t + 1.0, PISTILS[k], 0.0, 0.0);
        s.fan_cake(t + 0.5, silver, -55.0, 3, 15.0);
        s.fan_cake(t + 0.5, silver, 55.0, 3, 15.0);
    }
    s.shot(76.0, &JIKANSA, 0.0, 0.0);
    s.shot(79.0, &YONDAN, 0.0, 0.0);
    s.bed(75.0, 88.0, &[Device::Mine(&COLOUR_MINE), strobe_mine], 1.5);
    s.star_mine(83.0, 88.0, RAINBOW, 70.0, 15.0, 0.15, 0.08);
    s.salvo(84.5, &YAEZAKI, &[-55.0, 55.0]);
    s.sweep(88.5, &[&SALUTE], -70.0, 70.0, 6, 0.1);
    for &x in &line {
        s.ground(89.0, strobe_mine, x, 0.0);
    }
    s.line(90.0, &KAMURO);
    s.finish(104.0)
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
