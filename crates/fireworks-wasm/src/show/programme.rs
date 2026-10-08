//! One programme for every venue.
//!
//! Each section is written once against the venue's fleet: chases run the
//! length of its line, fans fire from its barges, and feature shells open as
//! many abreast as the line has room for. A `Plan` sets only how long each
//! section runs and how dense it is, so the desktop and phone shows cannot
//! drift apart.
//!
//! Every layer of the sky stays busy: pearls, comets, mines, gerbs, and water
//! shells low; star-mine chases and pistils in the middle; a feature shell
//! high every few seconds. The finale is the densest minute.

use super::sheet::Sheet;
use super::Device;
use crate::designs::*;
use crate::shell::ShellDesign;

/// What differs between venues.
pub struct Plan {
    /// Rainbow chases in the colour waves, four seconds apart.
    pub waves: usize,
    /// Seconds between the showcase's feature shells.
    pub showcase: f64,
    /// Seconds between 4-gō shells in the showcase's middle layer.
    pub pulse: f64,
    /// Fan-cake hops in the pulse section, 1.6 s apart.
    pub hops: usize,
    /// Whether the gold-and-silver interlude plays before the finale.
    pub interlude: bool,
    /// Build-up chases in the finale, two seconds apart.
    pub builds: usize,
    /// Seconds of the finale's densest star mine.
    pub barrage: f64,
    /// Low effects a second under the whole show.
    pub bed: f64,
    /// How hard star mines and the finale's low layers fire, relative to
    /// five barges; a phone's smaller budget fills sooner.
    pub density: f64,
}

/// About three minutes across five barges.
pub const DESKTOP: Plan = Plan {
    waves: 7,
    showcase: 2.4,
    pulse: 0.9,
    hops: 20,
    interlude: true,
    builds: 7,
    barrage: 10.0,
    bed: 1.2,
    density: 1.0,
};

/// About a minute and three quarters across three barges, on a phone's
/// particle budget.
pub const MOBILE: Plan = Plan {
    waves: 5,
    showcase: 2.0,
    pulse: 1.2,
    hops: 10,
    interlude: false,
    builds: 4,
    barrage: 6.0,
    bed: 1.0,
    density: 0.55,
};

const PISTILS: [&ShellDesign; 4] = [&PISTIL_RED, &PISTIL_PINK, &PISTIL_PURPLE, &PISTIL_AQUA];
const MIDS: [&ShellDesign; 6] = [
    &GOLD_KIKU,
    &PISTIL_AQUA,
    &DOUBLE_RING,
    &PISTIL_PINK,
    &GOLD_KIKU,
    &PISTIL_PURPLE,
];

/// Seconds the last kamuro hang before the loop restarts.
const HANG: f64 = 16.5;

/// Write the programme on `s`, returning the length of one loop.
pub fn play(s: &mut Sheet, plan: &Plan) -> f64 {
    let mut t = overture(s);
    t = colour_waves(s, plan, t);
    t = showcase(s, plan, t);
    t = pulse(s, plan, t);
    if plan.interlude {
        t = gold_and_silver(s, t);
    }
    let last = finale(s, plan, t);
    // Under everything, a low layer that never stops.
    let pearls = [Device::Comet(&PEARL_RED), Device::Comet(&PEARL_AQUA)];
    s.bed(
        0.0,
        last,
        &[
            pearls[0],
            pearls[1],
            Device::Mine(&COLOUR_MINE),
            Device::Comet(&CRACKLE_COMET),
        ],
        plan.bed,
    );
    last + HANG
}

const SILVER: Device = Device::Comet(&SILVER_COMET);
const GOLD: Device = Device::Comet(&GOLD_COMET);
const CROSSETTES: Device = Device::Comet(&CROSSETTE_COMET);
const CRACKLE: Device = Device::Comet(&CRACKLE_COMET);
const STROBES: Device = Device::Comet(&STROBE_COMET);
const STROBE_MINES: Device = Device::Mine(&STROBE_MINE);

/// The outer barges, or the inner pair on a line of five or more.
fn flanks(line: &[f64]) -> (f64, f64) {
    let n = line.len();
    if n >= 5 {
        (line[1], line[n - 2])
    } else {
        (line[0], line[n - 1])
    }
}

/// Gerbs light the decks, comets race the line both ways, and the first
/// crowns open low, middle, and high at once.
fn overture(s: &mut Sheet) -> f64 {
    let line = s.barges();
    let edge = s.edge();
    let shots = 4 * line.len();
    let pace = 1.0 / shots as f64;
    s.gerbs(0.8, &SILVER_GERB, 2);
    s.chase(1.2, SILVER, -edge, edge, shots, pace, 0.0);
    s.chase(2.4, CRACKLE, edge, -edge, shots, pace, 0.0);
    for &x in &line {
        s.fan_cake(3.4, SILVER, x, 5, 28.0);
    }
    s.salvo(4.6, &PISTIL_AQUA, &line);
    s.feature(5.3, &YONDAN, 2);
    s.z_cake(5.8, GOLD, line[0], 40, 0.18, 35.0);
    s.z_cake(5.89, GOLD, line[line.len() - 1], 40, 0.18, -35.0);
    s.star_mine(6.0, 13.5, RAINBOW, edge * 0.5, 12.0, 0.3, 0.18);
    s.fan(7.5, &[&HEART], 0.0, 3, 10.0);
    s.feature(9.0, &BROCADE_TIPS, 3);
    s.feature(10.5, &DOUBLE_RING, 2);
    s.line(12.0, &SALUTE);
    for &x in &line {
        s.ground(12.0, STROBE_MINES, x, 0.0);
    }
    s.shot(13.2, &JIKANSA, 0.0, 0.0);
    14.0
}

/// Rainbow chases sweep the line in alternate directions, pistils answer, a
/// feature opens high every four seconds, and candles of pearls keep coming
/// below.
fn colour_waves(s: &mut Sheet, plan: &Plan, start: f64) -> f64 {
    let line = s.barges();
    let edge = s.edge();
    let chase = 3 * line.len();
    for k in 0..plan.waves {
        let t = start + 4.0 * k as f64;
        let (from, to) = if k % 2 == 0 {
            (-edge, edge)
        } else {
            (edge, -edge)
        };
        s.sweep(t, RAINBOW, from, to, chase, 1.5 / chase as f64);
        s.fan(
            t + 2.0,
            &[PISTILS[k % 4]],
            line[(k * 2) % line.len()],
            3,
            15.0,
        );
        let at = t + 2.0;
        match k {
            0 => s.feature(at, &SLIDE, 1),
            1 => s.fan(at, &[&BUTTERFLY], 0.0, 3, 12.0),
            2 => s.feature(at, &JIKANSA, 2),
            3 => {
                let (left, right) = flanks(&line);
                s.fan(at, &[&STAR], left, 2, 10.0);
                s.fan(at, &[&SMILEY], right, 2, 10.0);
            }
            4 => s.feature(at, &YONDAN, 1),
            5 => s.feature(at, &SATURN, 2),
            _ => s.feature(at, &GHOST, 1),
        }
    }
    let end = start + 4.0 * plan.waves as f64 + 2.0;
    let mut k = 0;
    while start + 2.0 * (k as f64) < end - 2.0 {
        let x = line[k % line.len()];
        let lean = -x / edge * 8.0;
        s.candle(start + 2.0 * k as f64, PEARLS, x, 6, 0.33, lean);
        k += 1;
    }
    s.bed(
        start,
        end,
        &[Device::Mine(&COLOUR_MINE), CRACKLE, SILVER],
        1.4,
    );
    end
}

/// The signature shells and modern styles high, one every few seconds, over
/// a steady pulse of 4-gō shells, water fans, mines, and gold gerbs.
fn showcase(s: &mut Sheet, plan: &Plan, start: f64) -> f64 {
    let line = s.barges();
    let edge = s.edge();
    let n = line.len();
    let features: [(&'static ShellDesign, usize); 15] = [
        (&KAMURO, 1),
        (&TWINKLING, 2),
        (&SENRIN, 1),
        (&KOI, 2),
        (&DRAGON_EGGS, 1),
        (&TIME_RAIN_SHELL, 1),
        (&LEAVES, 2),
        (&BEES, 2),
        (&PALM, 2),
        (&CROSSETTE, 0),
        (&SPIDER, 3),
        (&HORSETAIL, 2),
        (&YAEZAKI, 3),
        (&GHOST, 2),
        (&SATURN, 1),
    ];
    for (i, &(design, wanted)) in features.iter().enumerate() {
        let at = start + 0.5 + plan.showcase * i as f64;
        if wanted == 0 {
            // Crossettes fan from the centre barge, one tube per barge.
            s.fan(at, &[design], 0.0, n, 22.0);
        } else {
            s.feature(at, design, wanted);
        }
    }
    let end = start + 0.5 + plan.showcase * features.len() as f64 + 1.0;
    let mut t = start;
    let mut k = 0;
    while t < end {
        let x = line[(k * 3) % n];
        let tilt = -x / edge * 8.0 + s.rng.range(-5.0, 5.0);
        s.shot(t, MIDS[k % MIDS.len()], x, tilt);
        t += plan.pulse;
        k += 1;
    }
    let fans = ((end - start) * 0.4 / 0.6) as usize;
    s.water_fans(start, fans, 0.6);
    s.bed(
        start + (end - start) * 0.35,
        end,
        &[
            Device::Mine(&SILVER_MINE),
            Device::Mine(&CRACKLE_MINE),
            GOLD,
        ],
        1.2,
    );
    s.gerbs(start + (end - start) * 0.5, &GOLD_GERB, 2);
    end
}

/// Fan cakes hop between barges over snaking Z cakes, V chases open from the
/// centre, colpi climb in steps, and an accelerating star mine ends on a wall
/// of salutes.
fn pulse(s: &mut Sheet, plan: &Plan, start: f64) -> f64 {
    let line = s.barges();
    let edge = s.edge();
    let n = line.len();
    let length = 1.6 * plan.hops as f64;
    let rack = if n >= 5 { 7 } else { 5 };
    for k in 0..plan.hops {
        let device = if k % 2 == 0 { CROSSETTES } else { SILVER };
        // Hop outside-in: ends first, centre last.
        let hop = [0, n - 1, 1.min(n - 1), n.saturating_sub(2), n / 2][k % 5];
        s.fan_cake(start + 1.6 * k as f64, device, line[hop], rack, 32.0);
    }
    let (left, right) = flanks(&line);
    let zig = (length / 0.15) as usize;
    s.z_cake(start, GOLD, left, zig, 0.15, 30.0);
    s.z_cake(start + 0.07, GOLD, right, zig, 0.15, -30.0);
    let mut k = 0;
    while 2.0 + 4.0 * (k as f64) + 1.0 < length {
        let t = start + 2.0 + 4.0 * k as f64;
        s.sweep(t, RAINBOW, 0.0, edge, n + 3, 0.12);
        s.sweep(t, RAINBOW, 0.0, -edge, n + 3, 0.12);
        k += 1;
    }
    let colpi = ((length - 4.0) / 4.0) as usize;
    for k in 0..colpi.min(5) {
        s.shot(
            start + 4.0 + 4.0 * k as f64,
            &COLPI,
            line[(k * 2 + 1) % n],
            0.0,
        );
    }
    s.feature(start + 0.3 * length, &BROCADE_TIPS, 2);
    s.feature(start + 0.45 * length, &YONDAN, 1);
    s.line(start + 0.6 * length, &SALUTE);
    s.star_mine(
        start + 0.62 * length,
        start + length - 0.5,
        STAR_MINE,
        edge,
        15.0,
        0.25 / plan.density,
        0.1 / plan.density,
    );
    s.bed(
        start + 0.62 * length,
        start + length,
        &[STROBE_MINES, STROBES],
        2.0 * plan.density,
    );
    s.line(start + length, &SALUTE);
    for &x in &line {
        s.ground(start + length, STROBE_MINES, x, 0.0);
    }
    start + length + 0.5
}

/// Hanging crowns, time rain, and leaves over water fans, willow mines, and
/// gerbs: a breath before the finale.
fn gold_and_silver(s: &mut Sheet, start: f64) -> f64 {
    s.feature(start + 1.0, &KAMURO, 2);
    s.shot(start + 2.0, &HORSETAIL, 0.0, 0.0);
    s.feature(start + 5.0, &TIME_RAIN_SHELL, 2);
    s.shot(start + 7.0, &LEAVES, 0.0, 0.0);
    s.feature(start + 9.0, &BROCADE_TIPS, 3);
    s.fan(start + 11.0, &[&TWINKLING], 0.0, 3, 15.0);
    s.shot(start + 13.5, &SATURN, 0.0, 0.0);
    s.water_fans(start + 1.0, 18, 0.65);
    s.gerbs(start + 3.0, &SILVER_GERB, 2);
    s.bed(
        start - 1.2,
        start + 16.0,
        &[
            Device::Mine(&WILLOW_MINE),
            GOLD,
            Device::Comet(&PEARL_LEMON),
        ],
        1.4,
    );
    start + 16.0
}

/// Chases every two seconds with pistils and comet walls, the densest star
/// mine, crossette fans from every barge, a golden curtain, salute chases, a
/// strobe wall, and kamuro left hanging. Returns when the last shells burst.
fn finale(s: &mut Sheet, plan: &Plan, start: f64) -> f64 {
    let line = s.barges();
    let edge = s.edge();
    let chase = 3 * line.len();

    // Build.
    let build = 2.0 * plan.builds as f64;
    for k in 0..plan.builds {
        let t = start + 2.0 * k as f64;
        let (from, to) = if k % 2 == 0 {
            (-edge, edge)
        } else {
            (edge, -edge)
        };
        s.sweep(t, RAINBOW, from, to, chase, 1.2 / chase as f64);
        s.feature(t + 1.0, PISTILS[k % 4], 2);
        let comet = if k % 2 == 0 { SILVER } else { CRACKLE };
        for &x in &line {
            s.fan_cake(t + 0.5, comet, x, 3, 15.0);
        }
    }
    let modern: [(f64, &'static ShellDesign, usize); 5] = [
        (1.5, &YONDAN, 2),
        (4.0, &JIKANSA, 1),
        (6.5, &SLIDE, 2),
        (9.0, &YONDAN, 3),
        (11.5, &HEART, 0),
    ];
    for (at, design, wanted) in modern {
        if at < build {
            if wanted == 0 {
                s.fan(start + at, &[design], 0.0, 3, 12.0);
            } else {
                s.feature(start + at, design, wanted);
            }
        }
    }
    s.bed(
        start,
        start + build,
        &[
            Device::Mine(&COLOUR_MINE),
            Device::Mine(&CRACKLE_MINE),
            STROBES,
        ],
        2.5 * plan.density,
    );

    // Barrage.
    let t = start + build;
    let barrage = plan.barrage;
    let (first, last) = (0.12 / plan.density, 0.06 / plan.density);
    s.star_mine(t, t + barrage, RAINBOW, edge, 18.0, first, last);
    let mut fan = 1.0;
    while fan < barrage {
        for &x in &line {
            s.fan(t + fan, &[&CROSSETTE], x, 3, 16.0);
        }
        fan += 3.0;
    }
    s.feature(t + 2.0, &YAEZAKI, 3);
    if barrage > 7.0 {
        s.feature(t + 6.0, &YAEZAKI, 4);
    }
    s.bed(t, t + barrage, &[STROBE_MINES, SILVER], 3.0 * plan.density);
    s.sweep(
        t + barrage - 2.0,
        &[&SALUTE],
        -edge,
        edge,
        2 * line.len() - 1,
        0.1,
    );
    s.sweep(
        t + barrage - 0.8,
        &[&SALUTE],
        edge,
        -edge,
        2 * line.len() - 1,
        0.1,
    );

    // Golden curtain.
    let t = t + barrage;
    s.line(t + 1.0, &KAMURO);
    for &x in &line {
        s.fan_cake(t + 1.5, GOLD, x, 7, 30.0);
    }
    s.feature(t + 3.5, &TIME_RAIN_SHELL, 3);
    s.line(t + 6.0, &BROCADE_TIPS);
    s.bed(
        t,
        t + 8.0,
        &[Device::Mine(&WILLOW_MINE), GOLD],
        2.0 * plan.density,
    );

    // Closer.
    let t = t + 8.0;
    s.sweep(t + 0.5, &[&SALUTE], -edge, edge, 2 * line.len(), 0.08);
    s.sweep(t + 1.4, &[&SALUTE], edge, -edge, 2 * line.len(), 0.08);
    for &x in &line {
        s.ground(t + 1.0, STROBE_MINES, x, -10.0);
        s.ground(t + 1.0, STROBE_MINES, x, 10.0);
    }
    s.feature(t + 3.0, &YONDAN, 3);
    s.line(t + 3.5, &KAMURO);
    t + 3.5
}
