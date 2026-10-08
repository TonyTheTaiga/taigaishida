//! The cue sheet: the vocabulary display designers compose with. Every
//! verb places devices on the venue's barge decks and draws its variations
//! from the loop's own dice.

use super::{Cue, Device, Program, Venue};
use crate::comet::{Comet, Gerb};
use crate::designs::WATER_FAN;
use crate::fleet::{Barge, LENGTH};
use crate::particle::Vec3;
use crate::rng::Rng;
use crate::shell::ShellDesign;

/// A flower's diameter is about 1,050 times its shell's: the Japan
/// Fireworks Association lists 3-gō (86 mm) flowers at 60–70 m and 7-gō
/// (199 mm) at 170–220 m.
const FLOWER_PER_METRE: f64 = 1050.0;

/// A cue sheet: the vocabulary display designers compose with.
pub(super) struct Sheet<'a> {
    fleet: &'static [Barge],
    cues: Vec<Cue>,
    pub rng: &'a mut Rng,
}

impl<'a> Sheet<'a> {
    pub(super) fn new(venue: Venue, rng: &'a mut Rng) -> Self {
        Self {
            fleet: venue.fleet(),
            cues: Vec::new(),
            rng,
        }
    }

    /// Where the barges lie along the firing line.
    pub(super) fn barges(&self) -> Vec<f64> {
        self.fleet.iter().map(|barge| barge.x).collect()
    }

    /// How far the usable decks reach either side of centre.
    pub(super) fn edge(&self) -> f64 {
        self.fleet.iter().map(|b| b.x.abs()).fold(0.0, f64::max) + LENGTH * 0.5 - 2.5
    }

    /// Positions for up to `wanted` shells of `design`, spread evenly over
    /// the line: as many as can open side by side without their flowers
    /// overlapping, and at least one, at the centre.
    pub(super) fn across(&self, design: &ShellDesign, wanted: usize) -> Vec<f64> {
        let width = 2.0 * self.edge();
        let flower = design.diameter * FLOWER_PER_METRE;
        let count = wanted.min((width / flower) as usize).max(1);
        (0..count)
            .map(|i| -0.5 * width + (i as f64 + 0.5) * width / count as f64)
            .collect()
    }

    /// Feature shells bursting together, spread by `across`.
    pub(super) fn feature(&mut self, burst: f64, design: &'static ShellDesign, wanted: usize) {
        let xs = self.across(design, wanted);
        self.salvo(burst, design, &xs);
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

    pub(super) fn shot(&mut self, burst: f64, design: &'static ShellDesign, x: f64, tilt: f64) {
        let mortar = self.at(x);
        self.cues.push(Cue::shell(burst, design, mortar, tilt));
    }

    /// Several mortars fired so their shells burst as one.
    pub(super) fn salvo(&mut self, burst: f64, design: &'static ShellDesign, xs: &[f64]) {
        for &x in xs {
            self.shot(burst, design, x, 0.0);
        }
    }

    /// One shell from every barge.
    pub(super) fn line(&mut self, burst: f64, design: &'static ShellDesign) {
        for i in 0..self.fleet.len() {
            self.shot(burst, design, self.fleet[i].x, 0.0);
        }
    }

    /// One rack of mortars splayed evenly between ±`spread` degrees, leaned
    /// toward the centre of the line as crews angle their outer racks.
    pub(super) fn fan(
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
    pub(super) fn sweep(
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
    pub(super) fn star_mine(
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

    pub(super) fn ground(&mut self, time: f64, device: Device, x: f64, tilt: f64) {
        let mortar = self.at(x);
        self.cues.push(Cue::ground(time, device, mortar, tilt));
    }

    /// Single tubes fired one after another along the line: a pixel front.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn chase(
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
    pub(super) fn fan_cake(
        &mut self,
        time: f64,
        device: Device,
        x: f64,
        count: usize,
        spread: f64,
    ) {
        let mortar = self.at(x);
        for i in 0..count {
            let tilt = -spread + 2.0 * spread * i as f64 / (count.max(2) - 1) as f64;
            let tilt = self.lean(x) + tilt;
            self.cues.push(Cue::ground(time, device, mortar, tilt));
        }
    }

    /// A Z cake: one tube at a time, snaking between ±`swing` degrees.
    pub(super) fn z_cake(
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
    pub(super) fn candle(
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
    pub(super) fn gerbs(&mut self, time: f64, gerb: &'static Gerb, per_barge: usize) {
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
    pub(super) fn water_fans(&mut self, from: f64, count: usize, interval: f64) {
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
    pub(super) fn bed(&mut self, from: f64, to: f64, devices: &[Device], rate: f64) {
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
    pub(super) fn finish(mut self, duration: f64) -> Program {
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
