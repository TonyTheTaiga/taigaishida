//! Ten shells built only from compositions, layers, and payload geometry.
//! Nothing here sets a colour curve, lifetime, or trajectory directly: each
//! effect comes from what the stars are made of and how they are packed.
//!
//! Shell anatomy follows Japanese practice:
//! - outer diameter per size (gō) from the Japan Fireworks Association table
//!   published by MLIT: 4-gō 114 mm, 5-gō 142 mm, 6-gō 171 mm, 7-gō 199 mm.
//!   Burst heights are not set here: they follow from the lift charge;
//! - empty paper casings weighed by Ooki et al. (2006, Table 1): 0.141, 0.209,
//!   and 0.477 kg for 4-, 5-, and 7-gō (6-gō interpolated);
//! - warimono main stars, star counts, and perchlorate bursting charges from
//!   Shimizu Table 23 (5-gō: 180 × 13 mm, 135 g; 6-gō: 200 × 15 mm, 270 g;
//!   7-gō: 220 × 18 mm, 550 g), with cotton-seed cores weighing 0.9× the
//!   charge (Table 19);
//! - poka shells (palms, fish, comets) burst with a few grams of black powder
//!   and weigh about half as much (Shimizu §17, Table 24).

use crate::chemistry::*;
use crate::comet::{Comet, Gerb, Mine};
use crate::shell::{Ignition, Orientation, Pattern, Payload, ShellDesign};

const fn mm(composition: &'static Composition, thickness: f64) -> Layer {
    Layer {
        composition,
        thickness: thickness * 1e-3,
    }
}

/// Casing plus bursting-charge cores of a warimono shell.
const fn warimono_inert(casing: f64, burst_charge: f64) -> f64 {
    casing + 0.9 * burst_charge
}

/// Black-powder lift charges photographed at an Osaka display (Shimizu Table
/// 27: 5-inch 38–45 g, 6-inch 75–85 g, 7-inch 110–130 g). They run 5–8% of
/// the shell's weight at every size, so the lighter poka shells below are
/// given 6% of their own mass.
const LIFT_5: f64 = 0.0415;
/// Table 27 starts at 5 inches; the smaller lifts are set so standard 3- and
/// 4-gō shells reach the Association's 125 m and 165 m.
const LIFT_3: f64 = 0.0055;
const LIFT_4: f64 = 0.016;
const LIFT_6: f64 = 0.08;
const LIFT_7: f64 = 0.12;

const CASING_3: f64 = 0.070;
const CASING_4: f64 = 0.141;
const CASING_5: f64 = 0.209;
const CASING_6: f64 = 0.33;
const CASING_7: f64 = 0.477;

/// Effect shells whose load differs from the standard keep Table 23's ratio
/// of bursting charge to star mass (270 g per 0.548 kg in a 6-gō), so their
/// outer stars leave at the 63.5 m/s Shimizu measured.

// 1. Yae-zaki sandan-gawari, 7-gō: a three-petal chrysanthemum. The 220 outer
//    18 mm stars carry a charcoal tail, then change red → green → blue as each
//    layer burns through (3.6 s in all, within Shimizu's 2–4 s "fast stars").
//    Two inner petals and a white-into-red pistil open at smaller radii.
const KIKU_OUTER: StarRecipe = &[
    mm(&CHARCOAL_TAIL, 1.5),
    mm(&STRONTIUM_RED, 2.5),
    mm(&BARIUM_GREEN, 2.5),
    mm(&COPPER_BLUE, 2.5),
];
const KIKU_MIDDLE: StarRecipe = &[mm(&BARIUM_GREEN, 2.5), mm(&PURPLE, 3.5)];
const KIKU_PISTIL: StarRecipe = &[mm(&MAGNESIUM_WHITE, 1.5), mm(&STRONTIUM_RED, 3.0)];
pub const YAEZAKI: ShellDesign = ShellDesign {
    name: "Yae-zaki chrysanthemum",
    diameter: 0.199,
    inert_mass: warimono_inert(CASING_7, 0.55),
    burst_charge: 0.55,
    burst_heat: PERCHLORATE_BURST_HEAT,
    lift_charge: LIFT_7,
    payload: &[
        Payload::Stars {
            recipe: KIKU_OUTER,
            count: 220,
            pattern: Pattern::Sphere,
            position: 1.0,
        },
        Payload::Stars {
            recipe: KIKU_MIDDLE,
            count: 110,
            pattern: Pattern::Sphere,
            position: 0.62,
        },
        Payload::Stars {
            recipe: KIKU_PISTIL,
            count: 70,
            pattern: Pattern::Sphere,
            position: 0.36,
        },
    ],
    comet: &[],
    orientation: Orientation::Tumbling,
    ignition: Ignition::TimeFuse,
};

// 2. Kamuro (crown willow), 7-gō: a weakened bursting charge and long-burning
//    charcoal stars that live 8–10 s (Shimizu §18.3). Their fire dust falls at
//    its terminal velocity, leaving a hanging gold crown that drifts.
pub const KAMURO: ShellDesign = ShellDesign {
    name: "Kamuro golden crown",
    diameter: 0.199,
    inert_mass: warimono_inert(CASING_7, 0.35),
    burst_charge: 0.35,
    burst_heat: PERCHLORATE_BURST_HEAT,
    lift_charge: LIFT_7,
    payload: &[Payload::Stars {
        recipe: &[mm(&KAMURO_GOLD, 9.5)],
        count: 200,
        pattern: Pattern::Sphere,
        position: 1.0,
    }],
    comet: &[],
    orientation: Orientation::Tumbling,
    ignition: Ignition::TimeFuse,
};

// 3. Crossette, 4-gō: sixteen 25 mm titanium comets with a black-powder split
//    charge inside. When the flame reaches it, each comet's core breaks into
//    four smaller comets. The 35 g black-powder burst is an estimate.
pub const CROSSETTE: ShellDesign = ShellDesign {
    name: "Crossette",
    diameter: 0.114,
    inert_mass: CASING_4,
    burst_charge: 0.035,
    burst_heat: BLACK_POWDER_HEAT,
    lift_charge: 0.06 * 0.38,
    payload: &[Payload::Stars {
        recipe: &[
            mm(&TITANIUM_SILVER, 5.0),
            mm(&SPLIT_CHARGE, 0.5),
            mm(&TITANIUM_SILVER, 7.0),
        ],
        count: 16,
        pattern: Pattern::Sphere,
        position: 1.0,
    }],
    comet: &[],
    orientation: Orientation::Tumbling,
    ignition: Ignition::TimeFuse,
};

// 4. Saturn, 6-gō: a violet-into-blue planet of 13 mm stars with a ring of
//    13 mm gold glitter stars packed around the equator of the same shell.
//    Charge: 0.49 × (0.267 kg ring + 0.55² × 0.232 kg planet) = 166 g.
pub const SATURN: ShellDesign = ShellDesign {
    name: "Saturn",
    diameter: 0.171,
    inert_mass: warimono_inert(CASING_6, 0.166),
    burst_charge: 0.166,
    burst_heat: PERCHLORATE_BURST_HEAT,
    lift_charge: LIFT_6,
    payload: &[
        Payload::Stars {
            recipe: &[mm(&PURPLE, 2.5), mm(&COPPER_BLUE, 4.0)],
            count: 130,
            pattern: Pattern::Sphere,
            position: 0.55,
        },
        Payload::Stars {
            recipe: &[mm(&GLITTER_GOLD, 6.5)],
            count: 150,
            pattern: Pattern::Ring,
            position: 1.0,
        },
    ],
    comet: &[],
    orientation: Orientation::Tumbling,
    ignition: Ignition::TimeFuse,
};

// 5. Senrin (thousand flowers), 6-gō: a shell of small shells (Shimizu §18.1).
//    Forty-five 30 mm shells, each on its own time fuse, burst into small
//    flowers of 4 mm stars (Shimizu §15.7 uses 3–5 mm). The small shells'
//    casings and charges are estimates; the main charge is 0.49 × 0.376 kg.
macro_rules! small_flower {
    ($name:literal, $recipe:expr) => {
        ShellDesign {
            name: $name,
            diameter: 0.03,
            inert_mass: 0.006,
            burst_charge: 0.0008,
            burst_heat: BLACK_POWDER_HEAT,
            lift_charge: 0.0,
            payload: &[Payload::Stars {
                recipe: $recipe,
                count: 30,
                pattern: Pattern::Sphere,
                position: 1.0,
            }],
            comet: &[],
            orientation: Orientation::Tumbling,
            ignition: Ignition::TimeFuse,
        }
    };
}
const FLOWER_RED: ShellDesign = small_flower!(
    "Red flower",
    &[mm(&STRONTIUM_RED, 1.0), mm(&SODIUM_YELLOW, 1.0)]
);
const FLOWER_GREEN: ShellDesign = small_flower!(
    "Green flower",
    &[mm(&BARIUM_GREEN, 1.0), mm(&MAGNESIUM_WHITE, 1.0)]
);
const FLOWER_GLITTER: ShellDesign = small_flower!("Glitter flower", &[mm(&GLITTER_GOLD, 2.0)]);
pub const SENRIN: ShellDesign = ShellDesign {
    name: "Senrin thousand flowers",
    diameter: 0.171,
    inert_mass: warimono_inert(CASING_6, 0.185),
    burst_charge: 0.185,
    burst_heat: PERCHLORATE_BURST_HEAT,
    lift_charge: LIFT_6,
    payload: &[Payload::Shells {
        designs: &[&FLOWER_RED, &FLOWER_GREEN, &FLOWER_GLITTER],
        count: 45,
        position: 1.0,
        fuse: (0.8, 1.6),
    }],
    comet: &[],
    orientation: Orientation::Tumbling,
    ignition: Ignition::TimeFuse,
};

// 6. Koi, 5-gō poka: ninety pierced black-powder stars that vent gas through one
//    side. Thrust is mass flow times exhaust speed, and the tumbling nozzle
//    makes each one swim. A 4 g black-powder burst (Shimizu Table 24) opens
//    the shell gently.
pub const KOI: ShellDesign = ShellDesign {
    name: "Swimming koi",
    diameter: 0.142,
    inert_mass: CASING_5,
    burst_charge: 0.004,
    burst_heat: BLACK_POWDER_HEAT,
    lift_charge: 0.06 * 0.37,
    payload: &[
        Payload::Stars {
            recipe: &[mm(&FISH_FUEL, 6.0)],
            count: 90,
            pattern: Pattern::Sphere,
            position: 1.0,
        },
        Payload::Stars {
            recipe: &[mm(&BARIUM_GREEN, 5.0)],
            count: 40,
            pattern: Pattern::Sphere,
            position: 0.55,
        },
    ],
    comet: &[],
    orientation: Orientation::Tumbling,
    ignition: Ignition::TimeFuse,
};

// 7. Dragon eggs, 5-gō: the standard 180 × 13 mm load. A short gold tail, then
//    a bismuth-trioxide layer that sheds microstars which smoulder dark for a
//    moment and then pop.
pub const DRAGON_EGGS: ShellDesign = ShellDesign {
    name: "Dragon eggs",
    diameter: 0.142,
    inert_mass: warimono_inert(CASING_5, 0.135),
    burst_charge: 0.135,
    burst_heat: PERCHLORATE_BURST_HEAT,
    lift_charge: LIFT_5,
    payload: &[Payload::Stars {
        recipe: &[mm(&CHARCOAL_TAIL, 1.5), mm(&DRAGON_EGG, 5.0)],
        count: 180,
        pattern: Pattern::Sphere,
        position: 1.0,
    }],
    comet: &[],
    orientation: Orientation::Tumbling,
    ignition: Ignition::TimeFuse,
};

// 8. Twinkling crown, 6-gō: the standard 200 × 15 mm load. Silver titanium
//    tails give way to Shimizu's green twinkler, flashing at 3.1 Hz for about
//    six seconds as the stars fall.
pub const TWINKLING: ShellDesign = ShellDesign {
    name: "Twinkling crown",
    diameter: 0.171,
    inert_mass: warimono_inert(CASING_6, 0.27),
    burst_charge: 0.27,
    burst_heat: PERCHLORATE_BURST_HEAT,
    lift_charge: LIFT_6,
    payload: &[Payload::Stars {
        recipe: &[mm(&TITANIUM_SILVER, 1.5), mm(&TWINKLER_GREEN, 6.0)],
        count: 200,
        pattern: Pattern::Sphere,
        position: 1.0,
    }],
    comet: &[],
    orientation: Orientation::Tumbling,
    ignition: Ignition::TimeFuse,
};

// 9. Brocade palm, 5-gō: ten heavy 25 mm brocade comets (Shimizu's 7–8 s
//    "slow stars") with an inner ring of small ones and a rising gold tail.
//    Their size gives them long range and a slow, drooping fall. The 20 g
//    black-powder burst is an estimate.
pub const PALM: ShellDesign = ShellDesign {
    name: "Brocade palm",
    diameter: 0.142,
    inert_mass: CASING_5,
    burst_charge: 0.02,
    burst_heat: BLACK_POWDER_HEAT,
    lift_charge: 0.06 * 0.39,
    payload: &[
        Payload::Stars {
            recipe: &[mm(&BROCADE, 12.5)],
            count: 10,
            pattern: Pattern::Sphere,
            position: 1.0,
        },
        Payload::Stars {
            recipe: &[mm(&BROCADE, 6.0)],
            count: 24,
            pattern: Pattern::Sphere,
            position: 0.5,
        },
    ],
    comet: &[mm(&BROCADE, 9.0)],
    orientation: Orientation::Tumbling,
    ignition: Ignition::TimeFuse,
};

// 10. Ghost shell, 7-gō: dark delay layers separate three colours, so the
//     sphere vanishes and reappears larger in a new colour, twice.
pub const GHOST: ShellDesign = ShellDesign {
    name: "Ghost shell",
    diameter: 0.199,
    inert_mass: warimono_inert(CASING_7, 0.55),
    burst_charge: 0.55,
    burst_heat: PERCHLORATE_BURST_HEAT,
    lift_charge: LIFT_7,
    payload: &[Payload::Stars {
        recipe: &[
            mm(&PURPLE, 2.0),
            mm(&DARK_DELAY, 1.2),
            mm(&BARIUM_GREEN, 2.3),
            mm(&DARK_DELAY, 0.9),
            mm(&CALCIUM_ORANGE, 2.6),
        ],
        count: 220,
        pattern: Pattern::Sphere,
        position: 1.0,
    }],
    comet: &[],
    orientation: Orientation::Tumbling,
    ignition: Ignition::TimeFuse,
};

// Star-mine shells: small, quick, and fired in rapid sequences.

// 3-gō peonies (86 mm): Shimizu's standard 150 × 9 mm stars and 35 g of
// perchlorate charge (Table 23). Peony stars have no tail.
macro_rules! peony {
    ($name:literal, $recipe:expr) => {
        ShellDesign {
            name: $name,
            diameter: 0.086,
            inert_mass: warimono_inert(CASING_3, 0.035),
            burst_charge: 0.035,
            burst_heat: PERCHLORATE_BURST_HEAT,
            lift_charge: LIFT_3,
            payload: &[Payload::Stars {
                recipe: $recipe,
                count: 150,
                pattern: Pattern::Sphere,
                position: 1.0,
            }],
            comet: &[],
            orientation: Orientation::Tumbling,
            ignition: Ignition::TimeFuse,
        }
    };
}
pub const PEONY_RED: ShellDesign = peony!("Red peony", &[mm(&STRONTIUM_RED, 4.5)]);
pub const PEONY_GREEN: ShellDesign = peony!("Green peony", &[mm(&BARIUM_GREEN, 4.5)]);
pub const PEONY_BLUE: ShellDesign = peony!("Blue peony", &[mm(&COPPER_BLUE, 4.5)]);
pub const PEONY_VIOLET: ShellDesign = peony!("Violet peony", &[mm(&PURPLE, 4.5)]);
pub const PEONY_SILVER: ShellDesign = peony!("Silver peony", &[mm(&MAGNESIUM_WHITE, 4.5)]);
pub const PEONY_YELLOW: ShellDesign = peony!("Yellow peony", &[mm(&SODIUM_YELLOW, 4.5)]);
pub const PEONY_CHANGE: ShellDesign = peony!(
    "Red-to-green peony",
    &[mm(&STRONTIUM_RED, 2.0), mm(&BARIUM_GREEN, 2.5)]
);

// 4-gō gold chrysanthemum (kin-kiku, 114 mm): 165 × 11 mm all-charcoal stars
// and 80 g of charge, between Table 23's 3.5- and 5-inch standards.
pub const GOLD_KIKU: ShellDesign = ShellDesign {
    name: "Gold chrysanthemum",
    diameter: 0.114,
    inert_mass: warimono_inert(CASING_4, 0.08),
    burst_charge: 0.08,
    burst_heat: PERCHLORATE_BURST_HEAT,
    lift_charge: LIFT_4,
    payload: &[Payload::Stars {
        recipe: &[mm(&CHARCOAL_TAIL, 5.5)],
        count: 165,
        pattern: Pattern::Sphere,
        position: 1.0,
    }],
    comet: &[],
    orientation: Orientation::Tumbling,
    ignition: Ignition::TimeFuse,
};

/// Every star-mine shell.
pub const STAR_MINE: &[&ShellDesign] = &[
    &PEONY_RED,
    &PEONY_GREEN,
    &PEONY_BLUE,
    &PEONY_SILVER,
    &PEONY_VIOLET,
    &PEONY_YELLOW,
    &PEONY_CHANGE,
    &GOLD_KIKU,
];

/// The ten signature shells.
#[cfg(test)]
pub const ALL: &[&ShellDesign] = &[
    &YAEZAKI,
    &KAMURO,
    &CROSSETTE,
    &SATURN,
    &SENRIN,
    &KOI,
    &DRAGON_EGGS,
    &TWINKLING,
    &PALM,
    &GHOST,
];

// ─── Modern shells ──────────────────────────────────────────────────
// Styles from recent Japanese competitions and Western pyromusicals. Loads
// follow Table 23's standards for each size; where a design departs from a
// standard star, its charge keeps Table 23's ratio of charge to star mass.

const fn shell(
    name: &'static str,
    diameter: f64,
    casing: f64,
    burst_charge: f64,
    lift_charge: f64,
    payload: &'static [Payload],
) -> ShellDesign {
    ShellDesign {
        name,
        diameter,
        inert_mass: warimono_inert(casing, burst_charge),
        burst_charge,
        burst_heat: PERCHLORATE_BURST_HEAT,
        lift_charge,
        payload,
        comet: &[],
        orientation: Orientation::Tumbling,
        ignition: Ignition::TimeFuse,
    }
}

/// A poka shell: opened by a few grams of black powder, so its stars barely
/// leave the burst and their own behaviour shapes the flower.
const fn poka(
    name: &'static str,
    diameter: f64,
    casing: f64,
    burst_charge: f64,
    payload: &'static [Payload],
) -> ShellDesign {
    ShellDesign {
        name,
        diameter,
        inert_mass: casing,
        burst_charge,
        burst_heat: BLACK_POWDER_HEAT,
        lift_charge: 0.0,
        payload,
        comet: &[],
        orientation: Orientation::Tumbling,
        ignition: Ignition::TimeFuse,
    }
}

/// Six percent of the shell's own weight, as for the lighter shells above.
const fn lifted(mut design: ShellDesign) -> ShellDesign {
    design.lift_charge = 0.06 * mass_estimate(&design);
    design
}

/// The shell mass for lift sizing, computed in a const context from each
/// payload's packed count and a typical 1.55 g/cm³ star of its recipe.
const fn mass_estimate(design: &ShellDesign) -> f64 {
    let mut mass = design.inert_mass + design.burst_charge;
    let mut i = 0;
    while i < design.payload.len() {
        mass += match &design.payload[i] {
            Payload::Stars { recipe, count, .. } => *count as f64 * star_mass(recipe),
            Payload::Shells { designs, count, .. } => *count as f64 * mass_estimate(designs[0]),
        };
        i += 1;
    }
    mass
}

const fn star_mass(recipe: StarRecipe) -> f64 {
    let mut radius = 0.0;
    let mut i = 0;
    while i < recipe.len() {
        radius += recipe[i].thickness;
        i += 1;
    }
    1550.0 * 4.0 / 3.0 * std::f64::consts::PI * radius * radius * radius
}

const fn upright(mut design: ShellDesign) -> ShellDesign {
    design.orientation = Orientation::Upright;
    design
}

// Jikansa-botan (time-difference peony), 5-gō: four quarters of 13 mm stars,
// each behind a dark delay 0.36 s longer than the last, so the flower lights
// around the clock in red, lemon, aqua, and pink. Thinner colour layers on
// the later quarters let all four go out together.
pub const JIKANSA: ShellDesign = upright(shell(
    "Jikansa-botan",
    0.142,
    CASING_5,
    0.135,
    LIFT_5,
    &[
        Payload::Stars {
            recipe: &[mm(&STRONTIUM_RED, 6.5)],
            count: 45,
            pattern: Pattern::Sector { index: 0, of: 4 },
            position: 1.0,
        },
        Payload::Stars {
            recipe: &[mm(&DARK_DELAY, 0.9), mm(&LEMON, 5.6)],
            count: 45,
            pattern: Pattern::Sector { index: 1, of: 4 },
            position: 1.0,
        },
        Payload::Stars {
            recipe: &[mm(&DARK_DELAY, 1.8), mm(&AQUA, 4.7)],
            count: 45,
            pattern: Pattern::Sector { index: 2, of: 4 },
            position: 1.0,
        },
        Payload::Stars {
            recipe: &[mm(&DARK_DELAY, 2.7), mm(&PINK, 3.8)],
            count: 45,
            pattern: Pattern::Sector { index: 3, of: 4 },
            position: 1.0,
        },
    ],
));

// Slide-botan, 6-gō: six slices, each lighting 0.25 s after its neighbour for
// 0.6 s of aqua, so a band of light sweeps across the flower; a second
// dark delay brings it back the other way in pink.
macro_rules! slice {
    ($index:literal, $lead:literal, $gap:literal) => {
        Payload::Stars {
            recipe: &[
                mm(&DARK_DELAY, $lead),
                mm(&AQUA, 1.44),
                mm(&DARK_DELAY, $gap),
                mm(&PINK, 1.74),
            ],
            count: 32,
            pattern: Pattern::Slice {
                index: $index,
                of: 6,
            },
            position: 1.0,
        }
    };
}
pub const SLIDE: ShellDesign = upright(shell(
    "Slide-botan",
    0.171,
    CASING_6,
    0.27,
    LIFT_6,
    &[
        slice!(0, 0.01, 6.625),
        slice!(1, 0.625, 5.375),
        slice!(2, 1.25, 4.125),
        slice!(3, 1.875, 2.875),
        slice!(4, 2.5, 1.625),
        slice!(5, 3.125, 0.375),
    ],
));

// Yondan-gawari henka-giku, 7-gō: a four-core changing chrysanthemum. The
// outer petal burns tail, red, green, then white strobe; four nested cores
// open inside it, the innermost a white strobe.
pub const YONDAN: ShellDesign = shell(
    "Yondan henka-giku",
    0.199,
    CASING_7,
    0.55,
    LIFT_7,
    &[
        Payload::Stars {
            recipe: &[
                mm(&CHARCOAL_TAIL, 1.2),
                mm(&STRONTIUM_RED, 2.2),
                mm(&BARIUM_GREEN, 2.2),
                mm(&WHITE_STROBE, 3.4),
            ],
            count: 200,
            pattern: Pattern::Sphere,
            position: 1.0,
        },
        Payload::Stars {
            recipe: &[mm(&AQUA, 2.5), mm(&PINK, 3.0)],
            count: 90,
            pattern: Pattern::Sphere,
            position: 0.74,
        },
        Payload::Stars {
            recipe: &[mm(&LEMON, 2.5), mm(&PURPLE, 2.5)],
            count: 70,
            pattern: Pattern::Sphere,
            position: 0.55,
        },
        Payload::Stars {
            recipe: &[mm(&MAGNESIUM_WHITE, 1.5), mm(&AQUA, 2.5)],
            count: 50,
            pattern: Pattern::Sphere,
            position: 0.38,
        },
        Payload::Stars {
            recipe: &[mm(&WHITE_STROBE, 3.5)],
            count: 30,
            pattern: Pattern::Sphere,
            position: 0.22,
        },
    ],
);

// Strobe-pistil peonies, 4-gō: a colour petal around a shimmering white
// strobe core.
macro_rules! strobe_pistil {
    ($name:literal, $colour:expr) => {
        shell(
            $name,
            0.114,
            CASING_4,
            0.08,
            LIFT_4,
            &[
                Payload::Stars {
                    recipe: &[mm($colour, 5.5)],
                    count: 120,
                    pattern: Pattern::Sphere,
                    position: 1.0,
                },
                Payload::Stars {
                    recipe: &[mm(&WHITE_STROBE, 4.0)],
                    count: 45,
                    pattern: Pattern::Sphere,
                    position: 0.45,
                },
            ],
        )
    };
}
pub const PISTIL_RED: ShellDesign = strobe_pistil!("Red strobe pistil", &STRONTIUM_RED);
pub const PISTIL_AQUA: ShellDesign = strobe_pistil!("Aqua strobe pistil", &AQUA);
pub const PISTIL_PINK: ShellDesign = strobe_pistil!("Pink strobe pistil", &PINK);
pub const PISTIL_PURPLE: ShellDesign = strobe_pistil!("Purple strobe pistil", &PURPLE);

// Katamono (pattern shells), 4-gō: stars laid on a flat template. Makers fire
// several at once, since some will always open edge-on.
fn heart(s: f64) -> (f64, f64) {
    let t = s * std::f64::consts::TAU;
    let x = 16.0 * t.sin().powi(3);
    let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
    (x / 16.0, (y + 2.5) / 16.0)
}

fn smiley(s: f64) -> (f64, f64) {
    use std::f64::consts::{PI, TAU};
    if s < 0.56 {
        let a = s / 0.56 * TAU;
        (a.cos(), a.sin())
    } else if s < 0.72 {
        let side = if s < 0.64 { -0.36 } else { 0.36 };
        let a = ((s - 0.56) / 0.08).fract() * TAU;
        (side + 0.09 * a.cos(), 0.3 + 0.13 * a.sin())
    } else {
        let a = PI * (1.15 + 0.7 * (s - 0.72) / 0.28);
        (0.6 * a.cos(), -0.02 + 0.6 * a.sin())
    }
}

fn five_point(s: f64) -> (f64, f64) {
    let vertex = |k: f64| {
        let r = if k as i64 % 2 == 0 { 1.0 } else { 0.42 };
        let a = std::f64::consts::FRAC_PI_2 + k * std::f64::consts::TAU / 10.0;
        (r * a.cos(), r * a.sin())
    };
    let edge = s * 10.0;
    let (a, b) = (vertex(edge.floor()), vertex(edge.floor() + 1.0));
    let f = edge.fract();
    (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
}

/// Fay's butterfly curve, centred and scaled to unit size.
fn butterfly(s: f64) -> (f64, f64) {
    let t = s * std::f64::consts::TAU;
    let r = t.cos().exp() - 2.0 * (4.0 * t).cos() - (t / 12.0).sin().powi(5);
    (t.sin() * r / 2.9, (t.cos() * r - 0.66) / 2.9)
}

macro_rules! picture {
    ($name:literal, $shape:expr, $recipe:expr, $count:literal) => {
        upright(shell(
            $name,
            0.114,
            CASING_4,
            0.06,
            LIFT_4,
            &[Payload::Stars {
                recipe: $recipe,
                count: $count,
                pattern: Pattern::Template($shape),
                position: 1.0,
            }],
        ))
    };
}
pub const HEART: ShellDesign = picture!("Heart", heart, &[mm(&STRONTIUM_RED, 5.0)], 70);
pub const SMILEY: ShellDesign = picture!("Smiley", smiley, &[mm(&LEMON, 5.0)], 80);
pub const STAR: ShellDesign = picture!("Star", five_point, &[mm(&AQUA, 5.0)], 80);
pub const BUTTERFLY: ShellDesign = picture!("Butterfly", butterfly, &[mm(&PINK, 4.5)], 110);

// Double ring, 4-gō: aqua and pink rings in one plane around a lemon pistil.
pub const DOUBLE_RING: ShellDesign = shell(
    "Double ring",
    0.114,
    CASING_4,
    0.08,
    LIFT_4,
    &[
        Payload::Stars {
            recipe: &[mm(&AQUA, 5.0)],
            count: 60,
            pattern: Pattern::Ring,
            position: 1.0,
        },
        Payload::Stars {
            recipe: &[mm(&PINK, 5.0)],
            count: 45,
            pattern: Pattern::Ring,
            position: 0.62,
        },
        Payload::Stars {
            recipe: &[mm(&LEMON, 4.0)],
            count: 25,
            pattern: Pattern::Sphere,
            position: 0.25,
        },
    ],
);

// Spider, 5-gō: sixty heavy, fast-burning gold stars thrown hard. They
// draw straight, flat lines and burn out together.
pub const SPIDER: ShellDesign = shell(
    "Spider",
    0.142,
    CASING_5,
    0.18,
    LIFT_5,
    &[Payload::Stars {
        recipe: &[mm(&SPIDER_GOLD, 10.0)],
        count: 60,
        pattern: Pattern::Sphere,
        position: 1.0,
    }],
);

// Horsetail, 5-gō: a weak black-powder burst barely parts heavy crown stars,
// which fall together as one gold plume.
pub const HORSETAIL: ShellDesign = lifted(poka(
    "Horsetail",
    0.142,
    CASING_5,
    0.012,
    &[Payload::Stars {
        recipe: &[mm(&KAMURO_GOLD, 8.0)],
        count: 45,
        pattern: Pattern::Sphere,
        position: 1.0,
    }],
));

// Time rain, 6-gō: ninety large stars that drift down shedding big glitter
// droplets; each droplet sizzles dark, then flashes.
pub const TIME_RAIN_SHELL: ShellDesign = shell(
    "Time rain",
    0.171,
    CASING_6,
    0.2,
    LIFT_6,
    &[Payload::Stars {
        recipe: &[mm(&TIME_RAIN, 8.0)],
        count: 90,
        pattern: Pattern::Sphere,
        position: 1.0,
    }],
);

// Falling leaves, 5-gō: seventy coated flakes opened gently, which tumble
// down on their own drag over five seconds.
pub const LEAVES: ShellDesign = lifted(poka(
    "Falling leaves",
    0.142,
    CASING_5,
    0.01,
    &[Payload::Stars {
        recipe: &[mm(&LEAF_GOLD, 8.0)],
        count: 70,
        pattern: Pattern::Sphere,
        position: 1.0,
    }],
));

// Hachi (bees), 4-gō: the shell pops in two and seventy spinning pierced
// stars corkscrew away.
pub const BEES: ShellDesign = lifted(poka(
    "Bees",
    0.114,
    CASING_4,
    0.006,
    &[Payload::Stars {
        recipe: &[mm(&BEE_FUEL, 3.0)],
        count: 70,
        pattern: Pattern::Sphere,
        position: 1.0,
    }],
));

// Titanium salute, 4-gō: flash-powder pellets with titanium. A white flash
// in a halo of white sparks; the usual finale closer.
pub const SALUTE: ShellDesign = shell(
    "Titanium salute",
    0.114,
    CASING_4,
    0.06,
    LIFT_4,
    &[Payload::Stars {
        recipe: &[mm(&FLASH_TITANIUM, 3.0)],
        count: 40,
        pattern: Pattern::Sphere,
        position: 1.0,
    }],
);

// Colpi (Italian multi-break), 5-gō: a red break carries a green break,
// which carries a salute, each lighter than the last and lit by its own
// time fuse 0.7–0.8 s after the one before.
const COLPI_FINAL: ShellDesign = shell(
    "Colpi third break",
    0.08,
    0.03,
    0.03,
    0.0,
    &[Payload::Stars {
        recipe: &[mm(&FLASH_TITANIUM, 3.0)],
        count: 20,
        pattern: Pattern::Sphere,
        position: 1.0,
    }],
);
const COLPI_SECOND: ShellDesign = shell(
    "Colpi second break",
    0.11,
    0.06,
    0.05,
    0.0,
    &[
        Payload::Stars {
            recipe: &[mm(&BARIUM_GREEN, 4.5)],
            count: 80,
            pattern: Pattern::Sphere,
            position: 1.0,
        },
        Payload::Shells {
            designs: &[&COLPI_FINAL],
            count: 1,
            position: 0.0,
            fuse: (0.7, 0.8),
        },
    ],
);
pub const COLPI: ShellDesign = shell(
    "Colpi",
    0.142,
    CASING_5,
    0.1,
    LIFT_5,
    &[
        Payload::Stars {
            recipe: &[mm(&STRONTIUM_RED, 4.5)],
            count: 100,
            pattern: Pattern::Sphere,
            position: 1.0,
        },
        Payload::Shells {
            designs: &[&COLPI_SECOND],
            count: 1,
            position: 0.0,
            fuse: (0.7, 0.8),
        },
    ],
);

// Brocade crown with coloured tips, 6-gō: the standard 200 × 15 mm load of
// brocade stars finishing in red or aqua.
pub const BROCADE_TIPS: ShellDesign = shell(
    "Brocade crown with tips",
    0.171,
    CASING_6,
    0.27,
    LIFT_6,
    &[
        Payload::Stars {
            recipe: &[mm(&BROCADE, 5.5), mm(&STRONTIUM_RED, 2.0)],
            count: 100,
            pattern: Pattern::Sphere,
            position: 1.0,
        },
        Payload::Stars {
            recipe: &[mm(&BROCADE, 5.5), mm(&AQUA, 2.0)],
            count: 100,
            pattern: Pattern::Sphere,
            position: 0.97,
        },
    ],
);

// Suijō (water) shell, 3-gō size: lobbed from the barge, it floats and
// bursts on the surface as a half-dome of gold-tailed stars, like a
// peacock's fan.
pub const WATER_FAN: ShellDesign = ShellDesign {
    ignition: Ignition::Contact,
    // A gentle lob of about 22 m/s.
    lift_charge: 0.0003,
    ..shell(
        "Water fan",
        0.086,
        CASING_3,
        0.03,
        0.0,
        &[Payload::Stars {
            recipe: &[mm(&CHARCOAL_TAIL, 3.0), mm(&LEMON, 1.5)],
            count: 70,
            pattern: Pattern::Dome,
            position: 1.0,
        }],
    )
};

// 3-gō peonies in the modern colours.
pub const PEONY_AQUA: ShellDesign = peony!("Aqua peony", &[mm(&AQUA, 4.5)]);
pub const PEONY_PINK: ShellDesign = peony!("Pink peony", &[mm(&PINK, 4.5)]);
pub const PEONY_LEMON: ShellDesign = peony!("Lemon peony", &[mm(&LEMON, 4.5)]);

/// Peonies in every colour for star-mine chases.
pub const RAINBOW: &[&ShellDesign] = &[
    &PEONY_RED,
    &PEONY_PINK,
    &PEONY_YELLOW,
    &PEONY_LEMON,
    &PEONY_GREEN,
    &PEONY_AQUA,
    &PEONY_BLUE,
    &PEONY_VIOLET,
];

// ─── Comets, pearls, mines, and gerbs ───────────────────────────────

/// A comet's lift is about 3% of its weight, giving 80–95 m/s.
const fn comet(name: &'static str, recipe: StarRecipe) -> Comet {
    Comet {
        name,
        recipe,
        lift_charge: 0.027 * star_mass(recipe),
    }
}

/// 25 mm titanium comet: a white-hot head and a long silver tail.
pub const SILVER_COMET: Comet = comet("Silver comet", &[mm(&TITANIUM_SILVER, 12.5)]);
/// 25 mm charcoal comet: a heavy gold tail that droops at the top.
pub const GOLD_COMET: Comet = comet("Gold comet", &[mm(&CHARCOAL_TAIL, 12.5)]);
/// 25 mm crossette comet: it splits into four at about 1.4 s.
pub const CROSSETTE_COMET: Comet = comet(
    "Crossette comet",
    &[
        mm(&TITANIUM_SILVER, 5.0),
        mm(&SPLIT_CHARGE, 0.5),
        mm(&TITANIUM_SILVER, 7.0),
    ],
);
/// 20 mm crackling comet: gold, then dragon eggs popping all the way up.
pub const CRACKLE_COMET: Comet = comet(
    "Crackling comet",
    &[mm(&CHARCOAL_TAIL, 1.0), mm(&DRAGON_EGG, 9.0)],
);
/// 16 mm white strobe comet.
pub const STROBE_COMET: Comet = comet("Strobe comet", &[mm(&WHITE_STROBE, 6.0)]);

/// Pearls: 18 mm colour stars shot from candles at about 45 m/s, which burn
/// out near the top of their flight.
macro_rules! pearl {
    ($name:literal, $composition:expr) => {
        Comet {
            name: $name,
            recipe: &[mm($composition, 9.0)],
            lift_charge: 0.0068 * star_mass(&[mm($composition, 9.0)]),
        }
    };
}
pub const PEARL_RED: Comet = pearl!("Red pearl", &STRONTIUM_RED);
pub const PEARL_AQUA: Comet = pearl!("Aqua pearl", &AQUA);
pub const PEARL_PINK: Comet = pearl!("Pink pearl", &PINK);
pub const PEARL_LEMON: Comet = pearl!("Lemon pearl", &LEMON);
pub const PEARL_GREEN: Comet = pearl!("Green pearl", &BARIUM_GREEN);
pub const PEARL_PURPLE: Comet = pearl!("Purple pearl", &PURPLE);
pub const PEARLS: &[&Comet] = &[
    &PEARL_RED,
    &PEARL_PINK,
    &PEARL_LEMON,
    &PEARL_GREEN,
    &PEARL_AQUA,
    &PEARL_PURPLE,
];

/// A 40 mm gerb charge jets sparks for about 12 s.
pub const SILVER_GERB: Gerb = Gerb {
    name: "Silver gerb",
    recipe: &[mm(&GERB_SILVER, 20.0)],
};
pub const GOLD_GERB: Gerb = Gerb {
    name: "Gold gerb",
    recipe: &[mm(&GERB_GOLD, 20.0)],
};

/// Mine lifts throw their stars out at 55–70 m/s.
const fn mine(name: &'static str, payload: &'static [(StarRecipe, usize)], spread: f64) -> Mine {
    let mut mass = 0.0;
    let mut i = 0;
    while i < payload.len() {
        mass += payload[i].1 as f64 * star_mass(payload[i].0);
        i += 1;
    }
    Mine {
        name,
        payload,
        // ½·M·(62 m/s)² = η·m·Q
        lift_charge: mass * 62.0 * 62.0 / (2.0 * 0.0882 * BLACK_POWDER_HEAT),
        spread,
    }
}

pub const COLOUR_MINE: Mine = mine(
    "Colour mine",
    &[
        (&[mm(&STRONTIUM_RED, 4.5)], 20),
        (&[mm(&AQUA, 4.5)], 20),
        (&[mm(&LEMON, 4.5)], 20),
    ],
    0.2,
);
pub const CRACKLE_MINE: Mine = mine(
    "Crackling mine",
    &[(&[mm(&CHARCOAL_TAIL, 1.0), mm(&DRAGON_EGG, 4.0)], 50)],
    0.22,
);
pub const SILVER_MINE: Mine = mine("Silver mine", &[(&[mm(&TITANIUM_SILVER, 5.0)], 40)], 0.18);
pub const WILLOW_MINE: Mine = mine("Gold willow mine", &[(&[mm(&KAMURO_GOLD, 5.0)], 40)], 0.18);
pub const STROBE_MINE: Mine = mine("Strobe mine", &[(&[mm(&WHITE_STROBE, 3.5)], 50)], 0.25);

/// The modern styles every programme features.
#[cfg(test)]
pub const MODERN: &[&ShellDesign] = &[
    &JIKANSA,
    &SLIDE,
    &YONDAN,
    &PISTIL_RED,
    &PISTIL_PINK,
    &PISTIL_PURPLE,
    &PISTIL_AQUA,
    &HEART,
    &SMILEY,
    &STAR,
    &BUTTERFLY,
    &DOUBLE_RING,
    &SPIDER,
    &HORSETAIL,
    &TIME_RAIN_SHELL,
    &LEAVES,
    &BEES,
    &SALUTE,
    &COLPI,
    &BROCADE_TIPS,
    &WATER_FAN,
];
