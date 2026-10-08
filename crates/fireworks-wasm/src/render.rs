//! Packs simulated light into the buffers `src/lib/renderers/glow.ts` draws.
//!
//! Layouts, in f32 elements:
//! - point (8): x, y (grid cells), radius (CSS px), red, green, blue (sRGB
//!   0–255), intensity (linear, unbounded), kind;
//! - trail segment (10): x0, y0, x1, y1 (grid cells), width (CSS px), red,
//!   green, blue (sRGB 0–255), intensity, unused;
//! - mesh vertex (6): x, y (grid cells), red, green, blue (linear radiance),
//!   coverage. Triangles arrive back to front.
//!
//! Intensities are linear light, not display values: the renderer blooms,
//! scatters, reflects, and tone maps them.

use crate::chemistry::{blackbody, Rgb};
use crate::projection::Camera;
use crate::star::{Puff, PuffKind, Spark, Star};
use std::sync::OnceLock;

pub const POINT_STRIDE: usize = 8;
pub const TRAIL_STRIDE: usize = 10;
pub const MESH_STRIDE: usize = 6;

/// Point kinds the renderer distinguishes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// Burning stars and sparks: a hot core that blooms.
    Emitter = 0,
    /// Smoke: radius is the parcel's size, intensity its optical depth.
    Smoke = 1,
    /// A burst or muzzle flash lighting the air around it.
    Flash = 2,
    /// A drone's LED or a vessel's lamp: a steady, hard point.
    Lamp = 3,
}

/// Persistence of vision: how long a moving spark smears across the retina.
const EXPOSURE: f64 = 0.08;

#[derive(Default)]
pub struct Frame {
    pub points: Vec<f32>,
    pub trails: Vec<f32>,
    pub mesh: Vec<f32>,
    /// Cap on trail segments, so a dense finale cannot grow the buffer.
    pub max_segments: usize,
}

impl Frame {
    pub fn new(points: usize, segments: usize, vertices: usize) -> Self {
        Self {
            points: Vec::with_capacity(points * POINT_STRIDE),
            trails: Vec::with_capacity(segments * TRAIL_STRIDE),
            mesh: Vec::with_capacity(vertices * MESH_STRIDE),
            max_segments: segments,
        }
    }

    pub fn clear(&mut self) {
        self.points.clear();
        self.trails.clear();
        self.mesh.clear();
    }

    pub fn segments(&self) -> usize {
        self.trails.len() / TRAIL_STRIDE
    }

    pub fn point(&mut self, at: (f64, f64), radius: f64, rgb: Rgb, intensity: f64, kind: Kind) {
        self.points.extend_from_slice(&[
            at.0 as f32,
            at.1 as f32,
            radius as f32,
            rgb.0 * 255.0,
            rgb.1 * 255.0,
            rgb.2 * 255.0,
            intensity.max(0.0) as f32,
            kind as u8 as f32,
        ]);
    }

    /// Returns false once the segment budget is spent.
    pub fn segment(
        &mut self,
        from: (f64, f64),
        to: (f64, f64),
        width: f64,
        rgb: Rgb,
        intensity: f64,
    ) -> bool {
        if self.segments() >= self.max_segments {
            return false;
        }
        self.trails.extend_from_slice(&[
            from.0 as f32,
            from.1 as f32,
            to.0 as f32,
            to.1 as f32,
            width as f32,
            rgb.0 * 255.0,
            rgb.1 * 255.0,
            rgb.2 * 255.0,
            intensity.max(0.0) as f32,
            0.0,
        ]);
        true
    }

    /// One opaque triangle of a vessel, in grid cells, with linear radiance.
    pub fn triangle(&mut self, corners: [(f64, f64); 3], light: [f64; 3]) {
        for (x, y) in corners {
            self.mesh.extend_from_slice(&[
                x as f32,
                y as f32,
                light[0] as f32,
                light[1] as f32,
                light[2] as f32,
                1.0,
            ]);
        }
    }
}

/// Light written to the HDR buffers. Luminance spans four decades between a
/// cooling ember and a titanium comet. A camera records it with a gentler
/// response than the eye's brightness sense (L^0.4), about its square root;
/// the renderer's tone curve compresses the rest.
pub fn radiance(luminance: f64) -> f64 {
    luminance.max(0.0).powf(0.5)
}

/// Distant light dims and shrinks; it never vanishes entirely.
fn distance_dimming(ratio: f64) -> f64 {
    ratio.clamp(0.35, 1.0)
}

/// Sprite scale for perspective: far bursts shrink, near ones grow, within
/// what a pixel grid can show.
fn sprite_scale(ratio: f64) -> f64 {
    ratio.clamp(0.4, 2.5)
}

const LUT_MIN: f64 = 800.0;
const LUT_STEP: f64 = 25.0;
const LUT_SIZE: usize = 140;

/// Blackbody colours from 800 K to 4275 K, computed once.
pub fn spark_colour(temperature: f64) -> Rgb {
    static TABLE: OnceLock<Vec<Rgb>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        (0..LUT_SIZE)
            .map(|i| blackbody(LUT_MIN + i as f64 * LUT_STEP))
            .collect()
    });
    let index = ((temperature - LUT_MIN) / LUT_STEP)
        .round()
        .clamp(0.0, (LUT_SIZE - 1) as f64);
    table[index as usize]
}

pub fn star(star: &Star, camera: &Camera, frame: &mut Frame) {
    let Some((rgb, luminance)) = star.light() else {
        return;
    };
    let light = radiance(luminance);
    if light < 0.01 {
        return;
    }
    let p = star.body.position;
    let Some(mut head) = camera.project(p.x, p.y, p.z) else {
        return;
    };
    let ratio = head.2;
    let intensity = light * distance_dimming(ratio);
    let size = sprite_scale(ratio);
    frame.point(
        (head.0, head.1),
        (1.6 + 0.9 * light.sqrt().min(3.0)) * size,
        rgb,
        intensity,
        Kind::Emitter,
    );
    // The eye integrates the last ~0.13 s of motion into a short streak.
    for (age, tail) in star
        .trail
        .samples()
        .enumerate()
        .filter(|(age, _)| age % 2 == 1)
    {
        let Some(end) = camera.project(tail.x, tail.y, tail.z) else {
            return;
        };
        let fade = (1.0 - age as f64 / 9.0).powi(2);
        if !frame.segment(
            (head.0, head.1),
            (end.0, end.1),
            1.5 * size,
            rgb,
            intensity * fade * 0.45,
        ) {
            return;
        }
        head = end;
    }
}

pub fn spark(spark: &Spark, camera: &Camera, frame: &mut Frame) {
    let light = radiance(spark.luminance());
    if light < 0.02 {
        return;
    }
    let p = spark.body.position;
    let Some(head) = camera.project(p.x, p.y, p.z) else {
        return;
    };
    let rgb = spark_colour(spark.temperature);
    let ratio = head.2;
    let intensity = light * distance_dimming(ratio);
    let size = sprite_scale(ratio);
    let t = p.sub(spark.body.velocity.scale(EXPOSURE));
    if let Some(tail) = camera.project(t.x, t.y, t.z) {
        frame.segment(
            (head.0, head.1),
            (tail.0, tail.1),
            (0.9 + 0.4 * light.sqrt().min(3.0)) * size,
            rgb,
            intensity,
        );
    }
    // Glitter flashes and popping microstars outshine their own streak.
    if light > 1.6 {
        frame.point(
            (head.0, head.1),
            (1.2 + 0.6 * light.sqrt().min(4.0)) * size,
            rgb,
            intensity,
            Kind::Emitter,
        );
    }
}

pub fn puff(puff: &Puff, camera: &Camera, frame: &mut Frame) {
    let p = puff.body.position;
    let Some((x, y, ratio)) = camera.project(p.x, p.y, p.z) else {
        return;
    };
    match puff.kind {
        PuffKind::Smoke => {
            let density = puff.density();
            if density > 0.002 {
                frame.point(
                    (x, y),
                    puff.radius() * camera.pixels_per_metre(ratio),
                    Rgb(1.0, 1.0, 1.0),
                    density,
                    Kind::Smoke,
                );
            }
        }
        // The fireball of a burst: a few metres of glowing gas for a sixth of
        // a second, bright enough to light the smoke and water around it.
        PuffKind::Flash => frame.point(
            (x, y),
            (6.0 + 8.0 * puff.intensity) * camera.pixels_per_metre(ratio),
            Rgb(1.0, 0.9, 0.72),
            5.0 * puff.intensity * puff.fraction().powi(3),
            Kind::Flash,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chemistry;
    use crate::particle::Vec3;
    use crate::projection::Stage;

    #[test]
    fn spark_and_star_output_match_their_physical_light() {
        let camera = Camera::new(100, 60, Stage::DESKTOP);
        let mut frame = Frame::new(100, 100, 0);
        const RECIPE: &[chemistry::Layer] = &[chemistry::Layer {
            composition: &chemistry::STRONTIUM_RED,
            thickness: 0.004,
        }];
        let lit = Star::new(
            RECIPE,
            1.0,
            Vec3::new(10.25, 120.0, 15.0),
            Vec3::new(5.0, 0.0, 0.0),
        );
        star(&lit, &camera, &mut frame);
        assert_eq!(frame.points.len(), POINT_STRIDE);
        let (x, y, _) = camera.project(10.25, 120.0, 15.0).unwrap();
        assert_eq!(&frame.points[..2], &[x as f32, y as f32]);
        // A 4 mm red star is the reference luminance: radiance 1, undimmed
        // this close to the reference plane.
        assert!((frame.points[6] - 1.0).abs() < 0.05);
        assert_eq!(frame.points[7], Kind::Emitter as u8 as f32);

        let hot = Spark::new(
            chemistry::CHARCOAL_TAIL.sparks.as_ref().unwrap(),
            Vec3::new(0.0, 120.0, 0.0),
            Vec3::new(10.0, 0.0, 0.0),
        );
        frame.clear();
        spark(&hot, &camera, &mut frame);
        assert_eq!(frame.trails.len(), TRAIL_STRIDE);
        assert!(
            frame.trails[0] > frame.trails[2],
            "streak trails behind the motion"
        );
        assert!(
            frame.trails[6] < frame.trails[5],
            "charcoal sparks glow orange"
        );
    }

    #[test]
    fn brighter_light_is_never_clipped_and_segments_respect_the_budget() {
        assert!(radiance(100.0) > radiance(10.0));
        assert!(radiance(100.0) > 1.0);
        let mut frame = Frame::new(0, 2, 0);
        let white = Rgb(1.0, 1.0, 1.0);
        assert!(frame.segment((0.0, 0.0), (1.0, 1.0), 1.0, white, 5.0));
        assert!(frame.segment((0.0, 0.0), (1.0, 1.0), 1.0, white, 5.0));
        assert!(!frame.segment((0.0, 0.0), (1.0, 1.0), 1.0, white, 5.0));
        assert_eq!(frame.trails[8], 5.0);
    }
}
