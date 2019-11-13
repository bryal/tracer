use nalgebra_glm::{vec3, Vec3};
use noise::{NoiseFn, Perlin};
use std::time;

use crate::intersect::*;

const SCENE_SIZE: isize = 20;

pub type Scene = Vec<Sphere>;

pub fn build_scene(t0: time::Instant) -> Scene {
    let a = t0.elapsed().as_secs_f64() / 1.0;
    let p = Perlin::new();
    let mut scene = (-SCENE_SIZE..SCENE_SIZE)
        .flat_map(|x| {
            let x = x as f32;
            (-SCENE_SIZE..SCENE_SIZE).map(move |z| {
                let z = z as f32;
                let y = (x as f64 + a).sin() as f32
                    + p.get([x as f64, z as f64, a / 2.0]) as f32 / 2.0;
                Sphere {
                    centre: vec3(x, y, z),
                    radius: 0.4,
                    color: vec3(1.0, 0.0, 0.0),
                }
            })
        })
        .collect::<Vec<_>>();
    scene.push(Sphere {
        centre: vec3(0.0, -101.0, 0.0),
        radius: 100.0,
        color: vec3(0.3, 0.3, 0.3),
    });
    scene
}

pub fn closest_hit(ray: &Ray, scene: &[Sphere]) -> Option<Hit> {
    scene
        .iter()
        .flat_map(|obj| obj.intersect(ray))
        .min_by(|h1, h2| h1.t.partial_cmp(&h2.t).expect("sorting hits"))
}

pub fn any_hit(ray: &Ray, scene: &[Sphere]) -> Option<Hit> {
    scene.iter().flat_map(|obj| obj.intersect(ray)).next()
}

pub struct Sphere {
    centre: Vec3,
    radius: f32,
    color: Vec3,
}

impl Sphere {
    pub fn intersect(&self, ray: &Ray) -> Option<Hit> {
        let oc = ray.origin - self.centre;
        let a = ray.dir.dot(&ray.dir);
        let b = 2.0 * oc.dot(&ray.dir);
        let c = oc.dot(&oc) - self.radius * self.radius;
        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            None
        } else {
            let sdiscriminant = discriminant.sqrt();
            // Negative root here means it's behind us.
            let root0 = -b - sdiscriminant;
            let root1 = -b + sdiscriminant;
            let mr = match (root0 <= root1, root0 >= 0.0, root1 >= 0.0) {
                (true, true, _) => Some(root0),
                (false, _, true) => Some(root1),
                _ => None,
            };
            mr.map(|r| {
                let t = r / (2.0 * a);
                Hit {
                    t: t,
                    normal: (oc + t * ray.dir) / self.radius,
                    color: self.color,
                }
            })
        }
    }
}