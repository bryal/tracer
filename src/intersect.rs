use nalgebra_glm::Vec3;
use rand::prelude::*;

use crate::material::*;

pub struct Ray<'r> {
    pub origin: Vec3,
    pub dir: Vec3,
    pub bounces: u8,
    pub throughput: Vec3,
    pub rng: &'r mut SmallRng,
}

pub struct BasicRay {
    pub origin: Vec3,
    pub dir: Vec3,
}

pub struct Hit {
    pub t: f32,
    pub normal: Vec3,
    pub mat: Mat,
}
