//! Perspective camera for the shared XYZ fireworks simulation.

use crate::particle::Vec3;

/// Grid cells are 14 px wide and 18 px tall; vertical grid units are scaled so
/// a metre covers the same number of pixels in both directions.
const CELL_ASPECT: f64 = 14.0 / 18.0;

/// The patch of sky and fleet a show is composed for.
#[derive(Clone, Copy)]
pub struct Stage {
    /// Metres visible across the reference plane on wide screens.
    pub width: f64,
    /// Metres of sky visible above the water on tall screens.
    pub sky: f64,
}

/// Share of the frame given to sky; the water and beach fill the rest.
pub const SKY_FRACTION: f64 = 0.8;

impl Stage {
    /// Panoramic: the 480 m firing line plus its tugs, under the tops of
    /// 5- and 6-gō flowers. Larger shells open past the top of the frame, as
    /// they do for spectators close to the water.
    pub const DESKTOP: Stage = Stage {
        width: 520.0,
        sky: 300.0,
    };
    /// Portrait-first: three barges filling the width of a phone.
    pub const MOBILE: Stage = Stage {
        width: 190.0,
        sky: 300.0,
    };
}

pub struct Camera {
    center_x: f64,
    eye_height: f64,
    screen_y: f64,
    focal_length: f64,
    distance: f64,
    pitch: f64,
    /// Grid cells per metre at the reference plane.
    framing: f64,
}

impl Camera {
    pub fn new(cols: usize, rows: usize, stage: Stage) -> Self {
        let cols = cols.max(1) as f64;
        let rows = rows.max(1) as f64;
        // A natural ~45° field of view; framing the stage then puts the viewer
        // 450–750 m from the barges, where waterfront crowds gather, well
        // beyond NFPA 1123's 70 ft per inch of shell.
        let focal_length = cols.max(rows).max(60.0) * 1.2;
        // Fit the same physical stage on every viewport by moving the
        // viewer, not resizing shells.
        let framing = (cols / stage.width).min(rows * SKY_FRACTION / CELL_ASPECT / stage.sky);
        // Tilt up until the horizon sits SKY_FRACTION of the way down.
        let pitch = ((rows * (SKY_FRACTION - 0.5)) / (focal_length * CELL_ASPECT)).atan();
        Self {
            center_x: cols * 0.5,
            eye_height: 6.0,
            screen_y: rows * 0.5,
            focal_length,
            distance: focal_length / framing,
            pitch,
            framing,
        }
    }

    /// Project world metres into grid coordinates. World Y is altitude; positive Z
    /// recedes. The third value is perspective size relative to the reference plane.
    pub fn project(&self, x: f64, y: f64, z: f64) -> Option<(f64, f64, f64)> {
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return None;
        }
        let relative_y = y - self.eye_height;
        let depth =
            self.distance * self.pitch.cos() + z * self.pitch.cos() + relative_y * self.pitch.sin();
        if depth < self.focal_length * 0.1 {
            return None;
        }
        let scale = self.focal_length / depth;
        let camera_up = relative_y * self.pitch.cos() - (self.distance + z) * self.pitch.sin();
        Some((
            self.center_x + x * scale,
            self.screen_y - camera_up * scale * CELL_ASPECT,
            scale / self.framing,
        ))
    }

    /// The viewer's eye in world metres; the camera looks toward +Z.
    pub fn eye(&self) -> Vec3 {
        Vec3::new(0.0, self.eye_height, -self.distance)
    }

    /// CSS pixels covered by one metre at a point whose perspective ratio
    /// `project` returned. Grid cells are 14 px wide.
    pub fn pixels_per_metre(&self, ratio: f64) -> f64 {
        self.framing * ratio * 14.0
    }

    /// Grid row of the horizon: where the water meets the sky at infinity.
    pub fn horizon(&self) -> f64 {
        self.screen_y + self.focal_length * self.pitch.tan() * CELL_ASPECT
    }

    /// Grid row where the water surface crosses the reference plane under
    /// the barges. Bursts on that plane reflect about this line.
    pub fn waterline(&self) -> f64 {
        self.project(0.0, 0.0, 0.0)
            .map_or(self.horizon(), |(_, y, _)| y)
    }

    /// Convert a screen-grid target on the reference plane (z=0) to world metres.
    #[cfg(test)]
    pub fn world_at_screen(&self, x: f64, y: f64) -> (f64, f64) {
        let camera_up = (self.screen_y - y) / CELL_ASPECT;
        let ray_y = camera_up * self.pitch.cos() + self.focal_length * self.pitch.sin();
        let ray_z = -camera_up * self.pitch.sin() + self.focal_length * self.pitch.cos();
        let distance = self.distance / ray_z;
        (
            (x - self.center_x) * distance,
            self.eye_height + ray_y * distance,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizon_matches_the_rendered_water_line_below_elevated_bursts() {
        let camera = Camera::new(100, 60, Stage::DESKTOP);
        let horizon = camera.horizon();
        assert!(camera.waterline() > horizon);
        let near_plane = camera.project(0.0, 0.0, -100.0).unwrap().1;
        let far_plane = camera.project(0.0, 0.0, 100.0).unwrap().1;
        let shell = camera.project(0.0, 40.0, 0.0).unwrap().1;
        assert!((horizon - 60.0 * SKY_FRACTION).abs() < 1e-10);
        assert!(near_plane > far_plane);
        assert!(shell < horizon);
        assert!(camera.project(0.0, 0.0, 0.0).unwrap().1 > horizon);
    }

    #[test]
    fn metres_are_square_on_screen_and_perspective_shrinks_distance() {
        let camera = Camera::new(100, 60, Stage::DESKTOP);
        let origin = camera.project(0.0, 100.0, 0.0).unwrap();
        let right = camera.project(10.0, 100.0, 0.0).unwrap();
        let up = camera.project(0.0, 110.0, 0.0).unwrap();
        let px_x = (right.0 - origin.0) * 14.0;
        let px_y = (origin.1 - up.1) * 18.0;
        assert!((px_x / px_y - 1.0).abs() < 0.05);
        let far = camera.project(30.0, 20.0, 100.0).unwrap();
        let near = camera.project(30.0, 20.0, 0.0).unwrap();
        assert!(far.0 < near.0 && far.0 > 50.0);
        assert!(far.2 < near.2);
        assert!((near.2 - 1.0).abs() < 0.05);
    }

    #[test]
    fn every_viewport_frames_the_same_stage() {
        for stage in [Stage::DESKTOP, Stage::MOBILE] {
            for (cols, rows) in [(100, 60), (27, 46), (180, 50), (60, 21), (102, 50)] {
                let camera = Camera::new(cols, rows, stage);
                let (left, _) = camera.world_at_screen(0.0, rows as f64 * 0.5);
                let (_, top) = camera.world_at_screen(cols as f64 * 0.5, 0.0);
                assert!(-left * 2.0 >= stage.width * 0.98);
                assert!(top >= stage.sky * 0.9, "{cols}x{rows} sky {top}");
            }
        }
    }

    #[test]
    fn viewer_stands_where_spectators_do() {
        // Desktop, ultrawide, phone portrait, and phone landscape viewports.
        // An ultrawide window stands back to fit the same sky.
        for (cols, rows, stage) in [
            (91, 44, Stage::DESKTOP),
            (180, 50, Stage::DESKTOP),
            (27, 46, Stage::MOBILE),
            (60, 21, Stage::MOBILE),
        ] {
            let distance = Camera::new(cols, rows, stage).distance;
            assert!(
                (400.0..=1400.0).contains(&distance),
                "{cols}x{rows} viewer at {distance} m"
            );
        }
    }

    #[test]
    fn screen_targets_round_trip_to_the_reference_world_plane() {
        let camera = Camera::new(100, 60, Stage::DESKTOP);
        let (x, y) = camera.world_at_screen(80.0, 12.0);
        let projected = camera.project(x, y, 0.0).unwrap();
        assert!((projected.0 - 80.0).abs() < 1e-10);
        assert!((projected.1 - 12.0).abs() < 1e-10);
    }

    #[test]
    fn clips_camera_plane_and_invalid_coordinates() {
        let camera = Camera::new(100, 60, Stage::DESKTOP);
        assert!(camera.project(1.0, 1.0, -camera.distance).is_none());
        assert!(camera.project(1.0, 1.0, -camera.distance * 2.0).is_none());
        assert!(camera.project(f64::NAN, 1.0, 0.0).is_none());
        assert!(Camera::new(0, 0, Stage::DESKTOP)
            .project(0.0, 10.0, 0.0)
            .is_some());
    }
}
