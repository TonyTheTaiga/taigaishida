//! The firing fleet: deck barges moored in a line offshore, each with a tug
//! standing off one end.
//!
//! Real display barges run 60–90 m with 3 m of freeboard (Macy's fires from
//! 76 m deck barges). These are drawn at about half that scale, 36 m by 10 m,
//! so the vessels stay modest under the show. Mortars, cakes, and comet racks
//! sit on the decks, so every shell, comet, and mine leaves from a barge.
//!
//! The hulls are lit only by the show: each face gathers the light of every
//! burning star and flash by the inverse-square law and Lambert's cosine, over
//! a faint ambient sky.

use crate::chemistry::Rgb;
use crate::particle::Vec3;
use crate::projection::Camera;
use crate::render::{Frame, Kind};
use crate::star::{Puff, PuffKind, Star};

pub const LENGTH: f64 = 36.0;
pub const BEAM: f64 = 10.0;
/// Deck height above the water.
pub const FREEBOARD: f64 = 1.4;
/// Mesh budget: every vessel's camera-facing faces, two triangles each.
pub const MAX_VERTICES: usize = 4096;

/// Irradiance per unit of star luminance at one metre, chosen so a 6-gō
/// flower overhead lights a hull about as brightly as a full moon would.
const LIGHT_SCALE: f64 = 40.0;
/// Starlight and distant shore light on the hulls.
const AMBIENT: [f64; 3] = [0.0012, 0.0014, 0.0019];
/// A burst's fireball radiates like this many reference stars at its peak.
const FLASH_LUMINANCE: f64 = 400.0;
/// Light closer than this is spread over the deck rather than a point.
const NEAREST: f64 = 10.0;

pub struct Barge {
    /// Deck centre, world metres: x along the shore, z away from the audience.
    pub x: f64,
    pub z: f64,
    /// The tug lies off this end: -1 for left, +1 for right.
    pub tug: f64,
}

impl Barge {
    /// A mortar on deck: `along` and `across` run -1..1 over the usable deck.
    pub fn mortar(&self, along: f64, across: f64) -> Vec3 {
        Vec3::new(
            self.x + along.clamp(-1.0, 1.0) * (LENGTH * 0.5 - 2.5),
            FREEBOARD + 0.4,
            self.z + across.clamp(-1.0, 1.0) * (BEAM * 0.5 - 1.5),
        )
    }

    fn blocks(&self) -> Vec<Block> {
        let (x, z) = (self.x, self.z);
        let hull = Block::new(
            Vec3::new(x, 0.0, z),
            Vec3::new(LENGTH, FREEBOARD, BEAM),
            [0.09, 0.095, 0.1],
        );
        let deck = FREEBOARD;
        let mut blocks = vec![hull];
        // Mortar racks and cake boxes in four bays.
        for bay in [-0.66, -0.22, 0.22, 0.66] {
            blocks.push(Block::new(
                Vec3::new(x + bay * LENGTH * 0.5, deck, z),
                Vec3::new(6.0, 1.0, 7.0),
                [0.32, 0.26, 0.19],
            ));
        }
        // The firing-control container, at the end away from the tug.
        blocks.push(Block::new(
            Vec3::new(x - self.tug * (LENGTH * 0.5 - 3.2), deck, z + 1.5),
            Vec3::new(5.0, 2.4, 2.4),
            [0.42, 0.43, 0.42],
        ));
        // The tug: hull, wheelhouse, funnel, and mast.
        let heading = -self.tug;
        let tx = x + self.tug * (LENGTH * 0.5 + 8.5);
        blocks.push(Block::new(
            Vec3::new(tx, 0.0, z),
            Vec3::new(12.0, 1.1, 4.4),
            [0.2, 0.05, 0.04],
        ));
        blocks.push(Block::new(
            Vec3::new(tx + heading * 1.5, 1.1, z),
            Vec3::new(3.6, 2.6, 3.4),
            [0.5, 0.5, 0.47],
        ));
        blocks.push(Block::new(
            Vec3::new(tx - heading * 1.4, 1.1, z),
            Vec3::new(1.0, 2.6, 1.0),
            [0.06, 0.05, 0.05],
        ));
        blocks.push(Block::new(
            Vec3::new(tx + heading * 1.5, 3.7, z),
            Vec3::new(0.3, 3.4, 0.3),
            [0.3, 0.3, 0.3],
        ));
        blocks
    }

    /// Navigation and anchor lights: (position, colour, intensity).
    fn lamps(&self) -> [(Vec3, Rgb, f64); 6] {
        let heading = -self.tug;
        let tx = self.x + self.tug * (LENGTH * 0.5 + 8.5);
        let white = Rgb(1.0, 0.93, 0.8);
        // A sidelight shows only on its own side; the audience faces the
        // starboard (green) side of a tug heading +x and the port (red) side
        // of one heading -x.
        let side = if heading > 0.0 {
            Rgb(0.1, 1.0, 0.45)
        } else {
            Rgb(1.0, 0.1, 0.05)
        };
        [
            (
                Vec3::new(self.x - LENGTH * 0.5 + 0.6, FREEBOARD + 3.0, self.z),
                white,
                0.9,
            ),
            (
                Vec3::new(self.x + LENGTH * 0.5 - 0.6, FREEBOARD + 3.0, self.z),
                white,
                0.9,
            ),
            (Vec3::new(tx + heading * 1.5, 7.4, self.z), white, 1.1),
            (Vec3::new(tx + heading * 1.5, 6.4, self.z), white, 1.1),
            (Vec3::new(tx + heading * 2.8, 3.4, self.z - 1.8), side, 0.9),
            // The wheelhouse windows, lit from inside.
            (
                Vec3::new(tx + heading * 1.5, 2.9, self.z - 1.75),
                Rgb(1.0, 0.75, 0.45),
                0.35,
            ),
        ]
    }
}

pub const LAMPS_PER_BARGE: usize = 6;

/// An axis-aligned box standing on `base`.
struct Block {
    base: Vec3,
    size: Vec3,
    albedo: [f64; 3],
}

impl Block {
    fn new(base: Vec3, size: Vec3, albedo: [f64; 3]) -> Self {
        Self { base, size, albedo }
    }

    fn centre(&self) -> Vec3 {
        self.base.add(Vec3::new(0.0, self.size.y * 0.5, 0.0))
    }

    /// The five faces above the waterline: (normal index, four corners).
    fn faces(&self) -> [(usize, [Vec3; 4]); 5] {
        let h = Vec3::new(self.size.x * 0.5, 0.0, self.size.z * 0.5);
        let (x0, x1) = (self.base.x - h.x, self.base.x + h.x);
        let (y0, y1) = (self.base.y, self.base.y + self.size.y);
        let (z0, z1) = (self.base.z - h.z, self.base.z + h.z);
        let v = Vec3::new;
        [
            (
                FRONT,
                [v(x0, y0, z0), v(x1, y0, z0), v(x1, y1, z0), v(x0, y1, z0)],
            ),
            (
                BACK,
                [v(x1, y0, z1), v(x0, y0, z1), v(x0, y1, z1), v(x1, y1, z1)],
            ),
            (
                LEFT,
                [v(x0, y0, z1), v(x0, y0, z0), v(x0, y1, z0), v(x0, y1, z1)],
            ),
            (
                RIGHT,
                [v(x1, y0, z0), v(x1, y0, z1), v(x1, y1, z1), v(x1, y1, z0)],
            ),
            (
                TOP,
                [v(x0, y1, z0), v(x1, y1, z0), v(x1, y1, z1), v(x0, y1, z1)],
            ),
        ]
    }
}

const FRONT: usize = 0;
const BACK: usize = 1;
const LEFT: usize = 2;
const RIGHT: usize = 3;
const TOP: usize = 4;
const NORMALS: [Vec3; 5] = [
    Vec3 {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    },
    Vec3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    },
    Vec3 {
        x: -1.0,
        y: 0.0,
        z: 0.0,
    },
    Vec3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    },
    Vec3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    },
];

/// sRGB-encoded display colour to linear light.
fn linear(rgb: Rgb) -> [f64; 3] {
    [rgb.0, rgb.1, rgb.2].map(|c| (c as f64).max(0.0).powf(2.2))
}

/// Irradiance on each face orientation of each barge, linear RGB.
pub struct Light {
    barges: Vec<[[f64; 3]; 5]>,
}

impl Light {
    pub fn gather<'a>(
        fleet: &[Barge],
        stars: impl Iterator<Item = &'a Star>,
        puffs: &[Puff],
    ) -> Self {
        let mut barges = vec![[AMBIENT; 5]; fleet.len()];
        let mut add = |at: Vec3, rgb: [f64; 3], luminance: f64| {
            for (barge, light) in fleet.iter().zip(barges.iter_mut()) {
                let to = at.sub(Vec3::new(barge.x, FREEBOARD, barge.z));
                let distance = to.length().max(NEAREST);
                let falloff = LIGHT_SCALE * luminance / (distance * distance * distance);
                for (face, normal) in NORMALS.iter().enumerate() {
                    let cosine = (normal.x * to.x + normal.y * to.y + normal.z * to.z).max(0.0);
                    for channel in 0..3 {
                        light[face][channel] += rgb[channel] * falloff * cosine;
                    }
                }
            }
        };
        for star in stars {
            if let Some((rgb, luminance)) = star.light() {
                if luminance > 0.05 {
                    add(star.body.position, linear(rgb), luminance);
                }
            }
        }
        for puff in puffs.iter().filter(|p| p.kind == PuffKind::Flash) {
            add(
                puff.body.position,
                linear(Rgb(1.0, 0.9, 0.72)),
                FLASH_LUMINANCE * puff.intensity * puff.fraction().powi(3),
            );
        }
        Self { barges }
    }
}

pub fn render(fleet: &[Barge], light: &Light, camera: &Camera, frame: &mut Frame) {
    let eye = camera.eye();
    // Paint far vessels first, and within a vessel, far blocks first.
    let mut order: Vec<usize> = (0..fleet.len()).collect();
    order.sort_by(|&a, &b| {
        let da = Vec3::new(fleet[a].x, 0.0, fleet[a].z).sub(eye).length();
        let db = Vec3::new(fleet[b].x, 0.0, fleet[b].z).sub(eye).length();
        db.total_cmp(&da)
    });
    for index in order {
        let barge = &fleet[index];
        let irradiance = &light.barges[index];
        let mut blocks = barge.blocks();
        blocks.sort_by(|a, b| {
            let da = a.centre().sub(eye).length();
            let db = b.centre().sub(eye).length();
            db.total_cmp(&da)
        });
        for block in &blocks {
            for (face, corners) in block.faces() {
                let normal = NORMALS[face];
                let centre = corners
                    .iter()
                    .fold(Vec3::default(), |sum, c| sum.add(c.scale(0.25)));
                let toward = eye.sub(centre);
                if normal.x * toward.x + normal.y * toward.y + normal.z * toward.z <= 0.0 {
                    continue;
                }
                let Some(projected) = corners
                    .iter()
                    .map(|c| camera.project(c.x, c.y, c.z).map(|(x, y, _)| (x, y)))
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                let radiance = [0, 1, 2].map(|c| block.albedo[c] * irradiance[face][c]);
                if frame.mesh.len() / crate::render::MESH_STRIDE + 6 > MAX_VERTICES {
                    return;
                }
                frame.triangle([projected[0], projected[1], projected[2]], radiance);
                frame.triangle([projected[0], projected[2], projected[3]], radiance);
            }
        }
        for (at, colour, intensity) in barge.lamps() {
            if let Some((x, y, ratio)) = camera.project(at.x, at.y, at.z) {
                frame.point(
                    (x, y),
                    1.3 * ratio.clamp(0.6, 2.0),
                    colour,
                    intensity,
                    Kind::Lamp,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chemistry::{Layer, MAGNESIUM_WHITE};
    use crate::projection::Stage;

    const FLARE: &[Layer] = &[Layer {
        composition: &MAGNESIUM_WHITE,
        thickness: 0.01,
    }];

    fn barge() -> Barge {
        Barge {
            x: 0.0,
            z: 0.0,
            tug: 1.0,
        }
    }

    #[test]
    fn mortars_stand_on_the_deck() {
        let barge = barge();
        for (along, across) in [(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0), (5.0, -5.0)] {
            let mortar = barge.mortar(along, across);
            assert!(mortar.x.abs() < LENGTH * 0.5);
            assert!(mortar.z.abs() < BEAM * 0.5);
            assert!(mortar.y > FREEBOARD);
        }
    }

    #[test]
    fn bursts_light_the_hull_by_the_inverse_square_law() {
        let fleet = [barge()];
        let dark = Light::gather(&fleet, std::iter::empty(), &[]);
        let deck = |x: f64, y: f64, z: f64| Vec3::new(x, FREEBOARD + y, z);
        let near = Star::new(FLARE, 1.0, deck(0.0, 100.0, -50.0), Vec3::default());
        let far = Star::new(FLARE, 1.0, deck(0.0, 200.0, -100.0), Vec3::default());
        let lit_near = Light::gather(&fleet, std::iter::once(&near), &[]);
        let lit_far = Light::gather(&fleet, std::iter::once(&far), &[]);
        // Per unit of each star's own output, which flutters.
        let gain = |light: &Light, face: usize, star: &Star| {
            (light.barges[0][face][1] - dark.barges[0][face][1]) / star.light().unwrap().1
        };
        // Twice as far, a quarter of the light, on every face that sees it.
        let ratio = |face| gain(&lit_near, face, &near) / gain(&lit_far, face, &far);
        assert!((ratio(TOP) - 4.0).abs() < 1e-9);
        assert!((ratio(FRONT) - 4.0).abs() < 1e-9);
        // Light from above and in front reaches neither the back nor the ends.
        assert_eq!(gain(&lit_near, BACK, &near), 0.0);
        assert_eq!(gain(&lit_near, LEFT, &near), 0.0);
        assert!(gain(&lit_near, TOP, &near) > gain(&lit_near, FRONT, &near));
    }

    #[test]
    fn only_camera_facing_faces_are_drawn() {
        let camera = Camera::new(137, 50, Stage::DESKTOP);
        let fleet = [barge()];
        let light = Light::gather(&fleet, std::iter::empty(), &[]);
        let mut frame = Frame::new(64, 0, MAX_VERTICES);
        render(&fleet, &light, &camera, &mut frame);
        let vertices = frame.mesh.len() / crate::render::MESH_STRIDE;
        assert_eq!(vertices % 3, 0);
        // Ten blocks: the front and top of each, plus one end of the
        // off-centre tug blocks. Never all five faces.
        assert!(vertices <= 10 * 3 * 6, "{vertices}");
        assert!(vertices >= 10 * 2 * 6, "{vertices}");
        assert_eq!(frame.points.len() / 8, LAMPS_PER_BARGE);
        assert!(frame.mesh.iter().all(|v| v.is_finite()));
    }
}
