use nalgebra::{Matrix4, Point3, Vector3};

/// An ArcBall controller, as defined by Ken Shoemake.
/// See http://www.talisman.org/~erlkonig/misc/shoemake92-arcball.pdf
#[derive(Clone, Debug)]
pub struct ArcBall {
    mouse_pos: Point2<f32>,
    prev_mouse_pos: Point2<f32>,
    center: Point3<f32>,
    radius: f32,
}

impl ArcBall {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn with_radius(radius: f32) -> Self {
        ArcBall {
            mouse_pos: Point::origin(),
            prev_mouse_pos: Point::origin(),
            center: Point::origin(),
            radius,
        }
    }

    #[inline]
    pub fn on_mouse_move(&mut self, delta_mouse_x: f32, delta_mouse_y: f32) {
        self.prev_mouse_pos = self.mouse_pos;
        self.mouse_pos.coords.x += delta_mouse_x;
        self.mouse_pos.coords.y += delta_mouse_y;
    }

    pub fn on_scroll(&mut self, amount: f32) {}

    pub fn calculate_view_matrix(&self) -> Matrix4<f32> {
        unimplemented!()
    }
}

impl Default for ArcBall {
    #[inline]
    fn default() -> Self {
        ArcBall::with_radius(1.0)
    }
}
