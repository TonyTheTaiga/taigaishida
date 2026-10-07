//! Perspective camera for the shared XYZ fireworks simulation.

pub struct Camera {
    center_x: f64,
    eye_height: f64,
    screen_y: f64,
    focal_length: f64,
    distance: f64,
    pitch: f64,
}

impl Camera {
    // Lower framing keeps the show comfortably distant while preserving the
    // same normalized size response for particles, trails, and launchers.
    pub const FRAMING: f64 = 0.38;

    pub fn new(cols: usize, rows: usize) -> Self {
        let focal_length = (cols.max(rows) as f64).max(60.0) * 3.0;
        let pitch = ((rows as f64 * 0.25) / focal_length).atan();
        Self {
            center_x: cols as f64 * 0.5,
            eye_height: (rows as f64 * 0.12).max(4.0),
            screen_y: rows as f64 * 0.5,
            focal_length,
            distance: focal_length / Self::FRAMING,
            pitch,
        }
    }

    /// Project world metres into grid coordinates. World Y is altitude; positive Z recedes.
    pub fn project(&self, x: f64, y: f64, z: f64) -> Option<(f64, f64, f64)> {
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return None;
        }
        let relative_y = y - self.eye_height;
        let depth = self.distance * self.pitch.cos()
            + z * self.pitch.cos()
            + relative_y * self.pitch.sin();
        if depth < self.focal_length * 0.1 {
            return None;
        }
        let scale = self.focal_length / depth;
        let camera_up = relative_y * self.pitch.cos() - (self.distance + z) * self.pitch.sin();
        Some((
            self.center_x + x * scale,
            self.screen_y - camera_up * scale,
            scale,
        ))
    }

    /// Convert a screen-grid target on the reference plane (z=0) to world metres.
    pub fn world_at_screen(&self, x: f64, y: f64) -> (f64, f64) {
        let camera_up = self.screen_y - y;
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
    fn perspective_camera_places_the_reference_plane_below_elevated_bursts() {
        let camera = Camera::new(100, 60);
        let horizon = camera.screen_y + camera.focal_length * camera.pitch.tan();
        let near_plane = camera.project(0.0, 0.0, -100.0).unwrap().1;
        let far_plane = camera.project(0.0, 0.0, 100.0).unwrap().1;
        let shell = camera.project(0.0, 40.0, 0.0).unwrap().1;
        assert!((horizon - 45.0).abs() < 1e-10);
        assert!(near_plane > far_plane);
        assert!(shell < horizon);
        assert!(camera.project(0.0, 0.0, 0.0).unwrap().1 > horizon);
    }

    #[test]
    fn projection_is_centered_and_perspective_shrinks_distant_points() {
        let camera = Camera::new(100, 60);
        let near = camera.project(30.0, 20.0, 0.0).unwrap();
        let far = camera.project(30.0, 20.0, 100.0).unwrap();
        assert!((camera.distance - camera.focal_length / Camera::FRAMING).abs() < 1e-10);
        assert!(far.0 < near.0 && far.0 > 50.0);
        assert!(far.2 < near.2);
        assert_eq!(camera.project(0.0, 10.0, 0.0).unwrap().0, 50.0);
    }

    #[test]
    fn screen_targets_round_trip_to_the_reference_world_plane() {
        let camera = Camera::new(100, 60);
        let (x, y) = camera.world_at_screen(80.0, 12.0);
        let projected = camera.project(x, y, 0.0).unwrap();
        assert!((projected.0 - 80.0).abs() < 1e-10);
        assert!((projected.1 - 12.0).abs() < 1e-10);
    }

    #[test]
    fn clips_camera_plane_and_invalid_coordinates() {
        let camera = Camera::new(100, 60);
        assert!(camera.project(1.0, 1.0, -camera.distance).is_none());
        assert!(camera.project(1.0, 1.0, -camera.distance * 2.0).is_none());
        assert!(camera.project(f64::NAN, 1.0, 0.0).is_none());
    }
}
