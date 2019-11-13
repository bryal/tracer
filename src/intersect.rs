use nalgebra_glm::Vec3;

pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

pub struct Hit {
    pub t: f32,
    pub normal: Vec3,
    pub color: Vec3,
}
