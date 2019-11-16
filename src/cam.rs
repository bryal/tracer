use {
    nalgebra::{base::Unit, geometry::Rotation3},
    nalgebra_glm::{vec3, Vec2, Vec3},
};

const FOV: f32 = 80.0;
const MOUSE_SENSITIVITY: f32 = 1.8;

pub struct Cam {
    pub pos: Vec3,
    dir: Vec3,
}

impl Cam {
    pub fn new(pos: Vec3, target: Vec3) -> Self {
        let dir = (target - pos).normalize();
        Self { pos, dir }
    }

    /// Returns the point of the screen origin in world space, a vector along
    /// the x-axis of the screen, and a vector along the y-axis of the screen.
    pub fn screen_vecs(&self, w: f32, h: f32) -> (Vec3, Vec3, Vec3) {
        let world_up = Vec3::y();
        let cam_right = self.dir.cross(&world_up).normalize();
        let cam_up = cam_right.cross(&self.dir).normalize();
        let aspect_ratio = w / h;
        let f = FOV.to_radians() / 2.0;
        let a = self.dir * f.cos();
        let b = cam_up * f.sin();
        let c = cam_right * f.sin() * aspect_ratio;
        let screen_origin = a - c - b;
        let screen_x_axis = 2.0 * (a - b - screen_origin);
        let screen_y_axis = 2.0 * (a - c - screen_origin);
        (screen_origin, screen_x_axis, screen_y_axis)
    }

    pub fn move_forwards(&mut self, d: f32) {
        self.pos += vec3(self.dir.x, 0.0, self.dir.z).normalize() * d;
    }

    pub fn move_backwards(&mut self, d: f32) {
        self.move_forwards(-d)
    }

    pub fn move_right(&mut self, d: f32) {
        let cam_right = self.dir.cross(&world_up()).normalize();
        self.pos += cam_right * d;
    }

    pub fn move_left(&mut self, d: f32) {
        self.move_right(-d)
    }

    pub fn move_up(&mut self, d: f32) {
        self.pos += world_up() * d;
    }

    pub fn move_down(&mut self, d: f32) {
        self.move_up(-d)
    }

    pub fn mouse_rotate(&mut self, dp: Vec2) {
        let yaw = Rotation3::from_axis_angle(
            &Vec3::y_axis(),
            -MOUSE_SENSITIVITY * dp.x,
        );
        let pitch = Rotation3::from_axis_angle(
            &Unit::new_normalize(self.dir.cross(&world_up())),
            -MOUSE_SENSITIVITY * dp.y,
        );
        self.dir = pitch * yaw * self.dir;
    }
}

fn world_up() -> Vec3 {
    Vec3::y()
}
