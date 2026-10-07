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
use crate::shell::{Pattern, Payload, ShellDesign};

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
};

/// Colour peonies cycled through star mines.
pub const PEONIES: &[&ShellDesign] = &[
    &PEONY_RED,
    &PEONY_GREEN,
    &PEONY_BLUE,
    &PEONY_SILVER,
    &PEONY_VIOLET,
    &PEONY_YELLOW,
    &PEONY_CHANGE,
];

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
