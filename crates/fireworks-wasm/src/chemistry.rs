//! Pyrotechnic chemistry: the smallest unit the engine models.
//!
//! A *composition* is a pressed or rolled mixture of oxidiser, fuel, and
//! colour agent. It burns inward at a linear regression rate, emits light from
//! its molecular emitter (SrCl → red, BaCl → green, CuCl → blue, Na → yellow)
//! or from hot condensed particles, and may eject incandescent spark
//! particles. A *star* is a sphere built from concentric layers of
//! compositions, listed from the outside in. Everything a viewer sees,
//! including colour changes, tails, glitter, strobing, swimming, and
//! splitting, emerges from those layers burning through.
//!
//! Measured values cite their source; values marked *estimate* had no
//! published measurement and were chosen within the physically plausible range.
//! Sources:
//! - Ooki et al., "Burning and air resistance of fireworks stars",
//!   Sci. Tech. Energetic Materials 67(1) 43 (2006): star sizes, masses,
//!   burn rates (3.0–6.0 mm/s on a plate, 1.6× slower in flight), Cd ≈ 0.5.
//! - T. Shimizu, *Fireworks: The Art, Science and Technique* (1981):
//!   Table 19 (bursting-charge heats, star velocities), Table 22 (twinklers),
//!   Table 23 (standard warimono stars), flame and fire-dust temperatures.
//! - Sandia National Laboratories (2023), titanium particle combustion in air.

/// Display colour (sRGB-encoded, brightest channel 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb(pub f32, pub f32, pub f32);

/// Heats of explosion of bursting charges, J/kg (Shimizu Table 19:
/// 690 kcal/kg for perchlorate "KP" charges, 400 kcal/kg for black powder).
pub const PERCHLORATE_BURST_HEAT: f64 = 690.0 * 4184.0;
pub const BLACK_POWDER_HEAT: f64 = 400.0 * 4184.0;

/// Pasted stars weigh 1.5–1.9 g/cm³ (Ooki Table 1; Shimizu Table 19).
const STAR_DENSITY: f64 = 1550.0;

/// Incandescent particles thrown from a burning layer. Each simulated spark is
/// a representative of many real ones; `per_gram` sets how many are sampled.
pub struct SparkFuel {
    pub per_gram: f64,
    pub diameter: f64,
    pub density: f64,
    /// Specific heat of the glowing particle, J/(kg·K).
    pub heat_capacity: f64,
    /// Ejection speed relative to the parent star, m/s.
    pub eject_speed: f64,
    /// Optional dark smoulder before ignition (glitter, crackling microstars).
    pub delay: (f64, f64),
    pub smoulder_temperature: f64,
    /// Time the particle keeps burning at `burn_temperature` before it cools.
    pub burn_time: (f64, f64),
    pub burn_temperature: f64,
}

pub enum Effect {
    None,
    /// Oscillating combustion: the same mass burns in short, bright pulses.
    Strobe {
        hz: f64,
        duty: f64,
    },
    /// Gas jet from a pierced star; the nozzle tumbles so the star swims.
    Thrust {
        exhaust_speed: f64,
        wander: f64,
    },
    /// A split charge. When the flame reaches it, the remaining core breaks
    /// into equal fragments driven apart by the charge's own energy.
    Split {
        fragments: usize,
        efficiency: f64,
    },
}

/// Where a burning layer's visible light comes from. The render loop uses the
/// precomputed `Composition::color`; the spectrum documents it and the tests
/// recompute it.
#[cfg_attr(not(test), allow(dead_code))]
pub enum Light {
    /// Molecular and atomic emitter bands: (wavelength nm, relative peak).
    Bands(&'static [(f64, f64)]),
    /// Glowing condensed particles at this temperature, K.
    Incandescent(f64),
    /// No visible flame (delay and split layers).
    Dark,
}

pub struct Composition {
    pub density: f64,
    /// Linear regression rate of the burning surface in flight, m/s.
    pub burn_rate: f64,
    #[cfg_attr(not(test), allow(dead_code))]
    pub light: Light,
    /// `light` rendered through the eye (see `Light::colour`); stored so the
    /// render loop does no spectral work. A test keeps the two in sync.
    pub color: Rgb,
    /// Relative luminous intensity per unit of burning surface (estimate).
    pub luminosity: f64,
    pub sparks: Option<SparkFuel>,
    pub effect: Effect,
}

pub struct Layer {
    pub composition: &'static Composition,
    pub thickness: f64,
}

pub type StarRecipe = &'static [Layer];

pub fn recipe_radius(layers: &[Layer]) -> f64 {
    layers.iter().map(|layer| layer.thickness).sum()
}

fn sphere_volume(radius: f64) -> f64 {
    4.0 / 3.0 * std::f64::consts::PI * radius.max(0.0).powi(3)
}

/// Mass of a layered sphere scaled by `scale` after `burned` metres have
/// regressed from its outer surface.
pub fn remaining_mass(layers: &[Layer], scale: f64, burned: f64) -> f64 {
    let mut outer = recipe_radius(layers) * scale;
    let surface = outer - burned;
    let mut mass = 0.0;
    for layer in layers {
        let inner = outer - layer.thickness * scale;
        let top = outer.min(surface);
        if top > inner {
            mass +=
                layer.composition.density * (sphere_volume(top) - sphere_volume(inner.max(0.0)));
        }
        outer = inner;
    }
    mass
}

// ─── Emitter spectra ────────────────────────────────────────────────
// Band positions from flame and plasma spectroscopy; relative peaks are
// approximate. Shimizu §9.4 names the emitters and their hues.

/// SrCl B–X (634.7, 660.1, 673.0 nm) and SrOH (606, 646, 659, 682 nm): deep red.
const STRONTIUM: &[(f64, f64)] = &[
    (606.0, 0.4),
    (634.7, 0.6),
    (646.0, 0.4),
    (659.0, 0.3),
    (660.1, 1.0),
    (673.0, 0.8),
    (682.0, 0.3),
];
/// BaCl green bands at 513.7 and 524.2 nm.
const BARIUM: &[(f64, f64)] = &[(513.7, 0.7), (524.2, 1.0)];
/// CuCl band heads in the 414–453 nm system and weaker 476–488 nm bands:
/// Shimizu's "pretty violet blue".
const COPPER: &[(f64, f64)] = &[
    (428.0, 0.6),
    (435.0, 0.8),
    (443.0, 1.0),
    (452.0, 0.7),
    (476.0, 0.35),
    (488.0, 0.25),
];
/// Strontium and copper together give violet (Shimizu §9.5); the red bands
/// need about three times the copper's peak to balance, as SrCl's deep red
/// barely registers with the eye.
const STRONTIUM_COPPER: &[(f64, f64)] = &[
    (428.0, 0.6),
    (435.0, 0.8),
    (443.0, 1.0),
    (452.0, 0.7),
    (476.0, 0.35),
    (488.0, 0.25),
    (606.0, 1.2),
    (634.7, 1.8),
    (646.0, 1.2),
    (659.0, 0.9),
    (660.1, 3.0),
    (673.0, 2.4),
    (682.0, 0.9),
];
/// Sodium D lines, 589.0 and 589.6 nm.
const SODIUM: &[(f64, f64)] = &[(589.0, 1.0), (589.6, 0.5)];
/// CaCl (593, 618 nm) and CaOH (554, 622 nm): reddish orange.
const CALCIUM: &[(f64, f64)] = &[(554.0, 0.35), (593.0, 1.0), (618.0, 0.7), (622.0, 0.5)];

// ─── Spark fuels ────────────────────────────────────────────────────

/// Charcoal fire dust from black-powder tails: about 1500 °C, "dark red to
/// bright orange" (Shimizu §10.2).
pub const CHARCOAL_SPARKS: SparkFuel = SparkFuel {
    per_gram: 30.0,
    diameter: 0.00025,
    density: 600.0,
    heat_capacity: 1500.0,
    eject_speed: 4.0,
    delay: (0.0, 0.0),
    smoulder_temperature: 0.0,
    burn_time: (0.25, 0.7),
    burn_temperature: 1780.0,
};

/// Long-burning pine-charcoal fire dust that hangs in the air (kamuro, brocade).
const HANGING_SPARKS: SparkFuel = SparkFuel {
    per_gram: 90.0,
    diameter: 0.00035,
    density: 650.0,
    heat_capacity: 1500.0,
    eject_speed: 3.0,
    delay: (0.0, 0.0),
    smoulder_temperature: 0.0,
    burn_time: (0.9, 1.9),
    burn_temperature: 1750.0,
};

/// Titanium particles burn near 2400 K once in ambient air (Sandia 2023).
const TITANIUM_SPARKS: SparkFuel = SparkFuel {
    per_gram: 45.0,
    diameter: 0.0002,
    density: 4500.0,
    heat_capacity: 800.0,
    eject_speed: 9.0,
    delay: (0.0, 0.0),
    smoulder_temperature: 0.0,
    burn_time: (0.15, 0.4),
    burn_temperature: 2400.0,
};

/// Aluminium/sulfide glitter droplets: a dull smoulder, then a white flash
/// (timings and temperatures are estimates).
const GLITTER_SPARKS: SparkFuel = SparkFuel {
    per_gram: 60.0,
    diameter: 0.0005,
    density: 2200.0,
    heat_capacity: 1100.0,
    eject_speed: 2.0,
    delay: (0.15, 0.55),
    smoulder_temperature: 1250.0,
    burn_time: (0.03, 0.07),
    burn_temperature: 3000.0,
};

/// Bismuth-trioxide microstars (dragon eggs): dark until they pop (estimate).
const MICROSTARS: SparkFuel = SparkFuel {
    per_gram: 30.0,
    diameter: 0.0012,
    density: 2400.0,
    heat_capacity: 900.0,
    eject_speed: 7.0,
    delay: (0.25, 0.8),
    smoulder_temperature: 800.0,
    burn_time: (0.025, 0.045),
    burn_temperature: 3000.0,
};

/// Black-powder jet sparks from pierced fish stars.
const JET_SPARKS: SparkFuel = SparkFuel {
    per_gram: 140.0,
    diameter: 0.0002,
    density: 700.0,
    heat_capacity: 1500.0,
    eject_speed: 6.0,
    delay: (0.0, 0.0),
    smoulder_temperature: 0.0,
    burn_time: (0.06, 0.16),
    burn_temperature: 1900.0,
};

// ─── Compositions ───────────────────────────────────────────────────

const fn coloured(
    bands: &'static [(f64, f64)],
    color: Rgb,
    burn_rate: f64,
    luminosity: f64,
) -> Composition {
    Composition {
        density: STAR_DENSITY,
        burn_rate,
        light: Light::Bands(bands),
        color,
        luminosity,
        sparks: None,
        effect: Effect::None,
    }
}

/// Red peony star: 5.0 mm/s on a plate, about 3.1 mm/s in flight (Ooki).
pub const STRONTIUM_RED: Composition = coloured(STRONTIUM, Rgb(1.0, 0.0, 0.0), 0.0031, 1.0);
/// Green: not in Ooki's tables; set between the measured red and blue (estimate).
pub const BARIUM_GREEN: Composition = coloured(BARIUM, Rgb(0.0, 1.0, 0.227), 0.0026, 0.95);
/// Blue peony star: 2.07–2.36 mm/s fired (Ooki Table 4). Blue is the dimmest emitter.
pub const COPPER_BLUE: Composition = coloured(COPPER, Rgb(0.0, 0.0, 1.0), 0.0022, 0.6);
/// Violet peony star: 3.0 mm/s on a plate, about 1.9 mm/s in flight (Ooki).
pub const PURPLE: Composition = coloured(STRONTIUM_COPPER, Rgb(0.665, 0.0, 1.0), 0.0019, 0.75);
/// Sodium yellow (burn rate estimate).
pub const SODIUM_YELLOW: Composition = coloured(SODIUM, Rgb(1.0, 0.644, 0.0), 0.0028, 1.1);
/// Calcium orange (burn rate estimate).
pub const CALCIUM_ORANGE: Composition = coloured(CALCIUM, Rgb(1.0, 0.525, 0.0), 0.0028, 1.0);

/// High-temperature-class white: magnesium or aluminium with an oxidiser burns
/// at 2500–3500 °C (Shimizu §9.2), glowing from MgO particles. Silver peony
/// stars regress at 6.0 mm/s on a plate, 3.5–4.0 mm/s in flight (Ooki).
pub const MAGNESIUM_WHITE: Composition = Composition {
    density: STAR_DENSITY,
    burn_rate: 0.00375,
    light: Light::Incandescent(3700.0),
    color: Rgb(0.727, 0.808, 1.0),
    luminosity: 2.5,
    sparks: None,
    effect: Effect::None,
};

/// Black powder and charcoal: a dim star that leaves a gold tail. Burns at the
/// rate of Ooki's silver crown star in flight, about 2.1 mm/s.
pub const CHARCOAL_TAIL: Composition = Composition {
    density: STAR_DENSITY,
    burn_rate: 0.0021,
    light: Light::Incandescent(1900.0),
    color: Rgb(1.0, 0.671, 0.395),
    luminosity: 0.3,
    sparks: Some(CHARCOAL_SPARKS),
    effect: Effect::None,
};

/// Crown-willow (kamuro) stars live 8–10 s (Shimizu §18.3); an 18–20 mm star
/// therefore regresses at about 1 mm/s.
pub const KAMURO_GOLD: Composition = Composition {
    density: STAR_DENSITY,
    burn_rate: 0.001,
    light: Light::Incandescent(1850.0),
    color: Rgb(1.0, 0.655, 0.365),
    luminosity: 0.22,
    sparks: Some(HANGING_SPARKS),
    effect: Effect::None,
};

/// Brocade palm comets are Shimizu's "slow stars" of 7–8 s (§2).
pub const BROCADE: Composition = Composition {
    density: STAR_DENSITY,
    burn_rate: 0.0016,
    light: Light::Incandescent(1900.0),
    color: Rgb(1.0, 0.671, 0.395),
    luminosity: 0.35,
    sparks: Some(SparkFuel {
        per_gram: 70.0,
        burn_time: (0.6, 1.3),
        burn_temperature: 1800.0,
        ..HANGING_SPARKS
    }),
    effect: Effect::None,
};

/// Perchlorate–titanium comet; metal-fuel stars burn like Ooki's silver peony.
pub const TITANIUM_SILVER: Composition = Composition {
    density: STAR_DENSITY,
    burn_rate: 0.0037,
    light: Light::Incandescent(3000.0),
    color: Rgb(0.99, 0.951, 1.0),
    luminosity: 0.6,
    sparks: Some(TITANIUM_SPARKS),
    effect: Effect::None,
};

pub const GLITTER_GOLD: Composition = Composition {
    density: STAR_DENSITY,
    burn_rate: 0.0018,
    light: Light::Incandescent(1900.0),
    color: Rgb(1.0, 0.671, 0.395),
    luminosity: 0.28,
    sparks: Some(GLITTER_SPARKS),
    effect: Effect::None,
};

/// Ammonium-perchlorate green twinkler, Shimizu Table 22: 3.1 Hz, 1.31 g/cm³,
/// 1.7 mm/s on a plate (≈1.06 mm/s in flight). The flash fraction of each
/// cycle is an estimate.
pub const TWINKLER_GREEN: Composition = Composition {
    density: 1310.0,
    burn_rate: 0.00106,
    light: Light::Bands(BARIUM),
    color: Rgb(0.0, 1.0, 0.227),
    luminosity: 1.4,
    sparks: None,
    effect: Effect::Strobe {
        hz: 3.1,
        duty: 0.25,
    },
};

pub const DRAGON_EGG: Composition = Composition {
    density: 1900.0,
    burn_rate: 0.0024,
    light: Light::Incandescent(1800.0),
    color: Rgb(1.0, 0.638, 0.334),
    luminosity: 0.12,
    sparks: Some(MICROSTARS),
    effect: Effect::None,
};

/// A non-luminous delay layer: the star flies on unseen.
pub const DARK_DELAY: Composition = Composition {
    density: STAR_DENSITY,
    burn_rate: 0.0025,
    light: Light::Dark,
    color: Rgb(0.0, 0.0, 0.0),
    luminosity: 0.0,
    sparks: None,
    effect: Effect::None,
};

/// Black-powder split charge; the share of its heat that becomes fragment
/// motion is an estimate.
pub const SPLIT_CHARGE: Composition = Composition {
    density: 1700.0,
    burn_rate: 0.02,
    light: Light::Dark,
    color: Rgb(0.0, 0.0, 0.0),
    luminosity: 0.0,
    sparks: None,
    effect: Effect::Split {
        fragments: 4,
        efficiency: 0.001,
    },
};

/// Black-powder fish fuel (Shimizu §21.4 "Chrysanthemum 6" with aluminium).
/// The effective exhaust speed of a pierced star is an estimate.
pub const FISH_FUEL: Composition = Composition {
    density: STAR_DENSITY,
    burn_rate: 0.0022,
    light: Light::Incandescent(1950.0),
    color: Rgb(1.0, 0.687, 0.425),
    luminosity: 0.5,
    sparks: Some(JET_SPARKS),
    effect: Effect::Thrust {
        exhaust_speed: 22.0,
        wander: 7.0,
    },
};

/// The shell's time fuse, burning for the whole climb (Wakabayashi et al.
/// 2006 measured 49–58 mm fuses on 170–235 mm shells; flights last 5–7 s).
pub const TIME_FUSE: Composition = Composition {
    density: 1500.0,
    burn_rate: 0.0008,
    light: Light::Incandescent(1850.0),
    color: Rgb(1.0, 0.655, 0.365),
    luminosity: 0.25,
    sparks: Some(SparkFuel {
        per_gram: 220.0,
        ..CHARCOAL_SPARKS
    }),
    effect: Effect::None,
};

// ─── Colour and light ───────────────────────────────────────────────

/// CIE 1931 2° colour-matching functions, from the multi-lobe Gaussian fit of
/// Wyman, Sloan & Shirley (JCGT 2013).
fn colour_matching(nm: f64) -> [f64; 3] {
    let lobe = |mu: f64, below: f64, above: f64| {
        let sigma = if nm < mu { below } else { above };
        (-0.5 * ((nm - mu) / sigma).powi(2)).exp()
    };
    [
        1.056 * lobe(599.8, 37.9, 31.0) + 0.362 * lobe(442.0, 16.0, 26.7)
            - 0.065 * lobe(501.1, 20.4, 26.2),
        0.821 * lobe(568.8, 46.9, 40.5) + 0.286 * lobe(530.9, 16.3, 31.1),
        1.217 * lobe(437.0, 11.8, 36.0) + 0.681 * lobe(459.0, 26.0, 13.8),
    ]
}

/// Planck spectral radiance at `nm`, normalised to 555 nm.
fn planck(nm: f64, temperature: f64) -> f64 {
    let c2 = 1.4388e7; // nm·K
    (555.0 / nm).powi(5) * (c2 / (555.0 * temperature)).exp_m1()
        / (c2 / (nm * temperature)).exp_m1()
}

fn tristimulus(spectrum: impl Fn(f64) -> f64, step: f64) -> [f64; 3] {
    let mut xyz = [0.0; 3];
    let mut nm = 360.0;
    while nm <= 830.0 {
        let power = spectrum(nm);
        let cmf = colour_matching(nm);
        for (total, weight) in xyz.iter_mut().zip(cmf) {
            *total += power * weight * step;
        }
        nm += step;
    }
    xyz
}

/// A show audience adapts toward the fireworks' own light: Shimizu (§8.2)
/// reports incandescence reading orange at 1250 °C and silver-white at
/// 2250 °C. Adaptation is partial in a dark surround; CIECAM02 gives
/// D = F·(1 − e^((−L_A − 42)/92)/3.6) ≈ 0.66 for F = 0.8 and L_A ≈ 1 cd/m².
const ADAPTED_WHITE: f64 = 2523.0;
const BRADFORD: [[f64; 3]; 3] = [
    [0.8951, 0.2664, -0.1614],
    [-0.7502, 1.7135, 0.0367],
    [0.0389, -0.0685, 1.0296],
];
const BRADFORD_INVERSE: [[f64; 3]; 3] = [
    [0.9869929, -0.1470543, 0.1599627],
    [0.4323053, 0.5183603, 0.0492912],
    [-0.0085287, 0.0400428, 0.9684867],
];

fn multiply(m: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|row| m[row][0] * v[0] + m[row][1] * v[1] + m[row][2] * v[2])
}

/// XYZ → partially adapted display sRGB, clipped to gamut and normalised.
fn display(xyz: [f64; 3]) -> Rgb {
    let degree = 0.8 * (1.0 - ((-1.0 - 42.0) / 92.0_f64).exp() / 3.6);
    let white = tristimulus(|nm| planck(nm, ADAPTED_WHITE), 5.0);
    let source = multiply(&BRADFORD, white.map(|c| c / white[1]));
    let target = multiply(&BRADFORD, [0.95047, 1.0, 1.08883]);
    let mut cone = multiply(&BRADFORD, xyz);
    for i in 0..3 {
        cone[i] *= degree * target[i] / source[i] + 1.0 - degree;
    }
    let [x, y, z] = multiply(&BRADFORD_INVERSE, cone);
    let linear = [
        3.2406 * x - 1.5372 * y - 0.4986 * z,
        -0.9689 * x + 1.8758 * y + 0.0415 * z,
        0.0557 * x - 0.2040 * y + 1.0570 * z,
    ]
    .map(|c| c.max(0.0));
    let peak = linear[0].max(linear[1]).max(linear[2]).max(1e-12);
    let encode = |c: f64| {
        let c = c / peak;
        (if c <= 0.0031308 {
            12.92 * c
        } else {
            1.055 * c.powf(1.0 / 2.4) - 0.055
        }) as f32
    };
    Rgb(encode(linear[0]), encode(linear[1]), encode(linear[2]))
}

impl Light {
    /// How the light looks to a dark-adapted viewer.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn colour(&self) -> Rgb {
        match *self {
            Light::Bands(bands) => display(tristimulus(
                |nm| {
                    bands
                        .iter()
                        .map(|&(centre, peak)| peak * (-0.5 * ((nm - centre) / 3.0).powi(2)).exp())
                        .sum()
                },
                0.5,
            )),
            Light::Incandescent(temperature) => blackbody(temperature),
            Light::Dark => Rgb(0.0, 0.0, 0.0),
        }
    }
}

/// Colour of a glowing grey body at `temperature`.
pub fn blackbody(temperature: f64) -> Rgb {
    display(tristimulus(|nm| planck(nm, temperature.max(500.0)), 5.0))
}

pub const REFERENCE_TEMPERATURE: f64 = 2200.0;

/// Photopic luminance of a grey body relative to one at 2200 K: the Planck
/// spectrum weighted by the eye's ȳ(λ) sensitivity, tabulated once.
pub fn incandescence(temperature: f64) -> f64 {
    use std::sync::OnceLock;
    const MIN: f64 = 500.0;
    const STEP: f64 = 25.0;
    const SIZE: usize = 160;
    static TABLE: OnceLock<Vec<f64>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let luminance = |t: f64| {
            // Absolute Planck radiance so different temperatures compare.
            tristimulus(
                |nm| planck(nm, t) * (1.4388e7 / (555.0 * t)).exp_m1().recip(),
                5.0,
            )[1]
        };
        let reference = luminance(REFERENCE_TEMPERATURE);
        (0..SIZE)
            .map(|i| luminance(MIN + i as f64 * STEP) / reference)
            .collect()
    });
    if temperature <= MIN {
        return 0.0;
    }
    let position = ((temperature - MIN) / STEP).min((SIZE - 1) as f64);
    let index = (position as usize).min(SIZE - 2);
    let fraction = position - index as f64;
    table[index] * (1.0 - fraction) + table[index + 1] * fraction
}

/// Stevens' power law: perceived brightness grows roughly with L^0.4.
pub fn perceived(luminance: f64) -> f64 {
    luminance.max(0.0).powf(0.4)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONION: &[Layer] = &[
        Layer {
            composition: &STRONTIUM_RED,
            thickness: 0.002,
        },
        Layer {
            composition: &COPPER_BLUE,
            thickness: 0.003,
        },
    ];

    const ALL: &[(&str, &Composition)] = &[
        ("STRONTIUM_RED", &STRONTIUM_RED),
        ("BARIUM_GREEN", &BARIUM_GREEN),
        ("COPPER_BLUE", &COPPER_BLUE),
        ("PURPLE", &PURPLE),
        ("SODIUM_YELLOW", &SODIUM_YELLOW),
        ("CALCIUM_ORANGE", &CALCIUM_ORANGE),
        ("MAGNESIUM_WHITE", &MAGNESIUM_WHITE),
        ("CHARCOAL_TAIL", &CHARCOAL_TAIL),
        ("KAMURO_GOLD", &KAMURO_GOLD),
        ("BROCADE", &BROCADE),
        ("TITANIUM_SILVER", &TITANIUM_SILVER),
        ("GLITTER_GOLD", &GLITTER_GOLD),
        ("TWINKLER_GREEN", &TWINKLER_GREEN),
        ("DRAGON_EGG", &DRAGON_EGG),
        ("DARK_DELAY", &DARK_DELAY),
        ("SPLIT_CHARGE", &SPLIT_CHARGE),
        ("FISH_FUEL", &FISH_FUEL),
        ("TIME_FUSE", &TIME_FUSE),
    ];

    #[test]
    fn layered_mass_matches_shell_geometry_and_scale() {
        let red = STRONTIUM_RED.density * (sphere_volume(0.005) - sphere_volume(0.003));
        let blue = COPPER_BLUE.density * sphere_volume(0.003);
        assert!((remaining_mass(ONION, 1.0, 0.0) - (red + blue)).abs() < 1e-12);
        assert!((remaining_mass(ONION, 1.0, 0.002) - blue).abs() < 1e-12);
        assert_eq!(remaining_mass(ONION, 1.0, 0.005), 0.0);
        let doubled = remaining_mass(ONION, 2.0, 0.0);
        assert!((doubled / remaining_mass(ONION, 1.0, 0.0) - 8.0).abs() < 1e-9);
    }

    #[test]
    fn stored_colours_match_their_spectra() {
        for (name, composition) in ALL {
            let derived = composition.light.colour();
            println!(
                "{name:16} Rgb({:.3}, {:.3}, {:.3})",
                derived.0, derived.1, derived.2
            );
        }
        for (name, composition) in ALL {
            let derived = composition.light.colour();
            let stored = composition.color;
            let close = |a: f32, b: f32| (a - b).abs() < 0.006;
            assert!(
                close(derived.0, stored.0)
                    && close(derived.1, stored.1)
                    && close(derived.2, stored.2),
                "{name} stores {stored:?} but its spectrum gives {derived:?}"
            );
        }
    }

    #[test]
    fn incandescence_follows_shimizus_colour_scale() {
        assert!((incandescence(REFERENCE_TEMPERATURE) - 1.0).abs() < 1e-9);
        assert!(incandescence(3000.0) > 10.0);
        assert!(incandescence(1000.0) < 1e-3);
        assert!(incandescence(1800.0) < incandescence(1900.0));
        // Orange at 1250 °C, near-white silver at 2250 °C.
        let orange = blackbody(1523.0);
        let silver = blackbody(2523.0);
        assert_eq!(orange.0, 1.0);
        assert!(orange.2 < 0.3 && orange.1 < 0.65);
        assert!(silver.1 > 0.8 && silver.2 > 0.7);
        // Strontium reads red, barium green, copper blue.
        let [red, green, blue] = [STRONTIUM, BARIUM, COPPER].map(|b| Light::Bands(b).colour());
        assert!(red.0 == 1.0 && red.1 < 0.2);
        assert!(green.1 == 1.0 && green.0 < 0.2);
        assert!(blue.2 == 1.0 && blue.1 < 0.2);
    }
}
