//! Shell-less devices fired from tubes on the barge decks.
//!
//! A *comet* is one large pressed star shot from its own tube; it burns all
//! the way up and leaves a tail. A *mine* is a mortar loaded with loose stars
//! and no shell: the lift charge throws them out of the muzzle as a spreading
//! cone. Cakes are racks of comet or mine tubes fused to fire one after
//! another; the show sheet sequences their tubes.
//!
//! Muzzle speed follows the same energy balance as the shells' lift,
//! ½·M·v² = η·m_lift·Q, with the mortar efficiency calibrated in `shell.rs`.

use crate::chemistry::{remaining_mass, StarRecipe, BLACK_POWDER_HEAT};
use crate::shell::LIFT_EFFICIENCY;

pub struct Comet {
    #[cfg_attr(not(test), allow(dead_code))]
    pub name: &'static str,
    pub recipe: StarRecipe,
    pub lift_charge: f64,
}

impl Comet {
    pub fn mass(&self) -> f64 {
        remaining_mass(self.recipe, 1.0, 0.0)
    }

    pub fn muzzle_speed(&self) -> f64 {
        (2.0 * LIFT_EFFICIENCY * self.lift_charge * BLACK_POWDER_HEAT / self.mass()).sqrt()
    }
}

pub struct Mine {
    #[cfg_attr(not(test), allow(dead_code))]
    pub name: &'static str,
    /// Star recipes and how many of each are loaded.
    pub payload: &'static [(StarRecipe, usize)],
    pub lift_charge: f64,
    /// Half-angle of the cone the stars leave in, radians.
    pub spread: f64,
}

impl Mine {
    pub fn mass(&self) -> f64 {
        self.payload
            .iter()
            .map(|(recipe, count)| *count as f64 * remaining_mass(recipe, 1.0, 0.0))
            .sum()
    }

    /// The whole load leaves together, like a shell without a casing.
    pub fn muzzle_speed(&self) -> f64 {
        (2.0 * LIFT_EFFICIENCY * self.lift_charge * BLACK_POWDER_HEAT / self.mass()).sqrt()
    }
}
