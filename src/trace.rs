use nalgebra_glm as glm;
use nalgebra_glm::{vec3, Vec3};
use rand::prelude::*;
use rayon::prelude::*;
use std::cmp;

use crate::cam::*;
use crate::geom::*;
use crate::intersect::*;
use crate::material::*;

type Pixel = (u8, u8, u8);

const RAY_EPSILON: f32 = 0.0001;
const MAX_BOUNCES: u8 = 3;

pub const ERR_COLOR_F: (f32, f32, f32) = (1.0, 0.0, 1.0);
pub const ERR_COLOR: Pixel = (
    (ERR_COLOR_F.0 * 255.0) as u8,
    (ERR_COLOR_F.1 * 255.0) as u8,
    (ERR_COLOR_F.2 * 255.0) as u8,
);

fn background_color() -> Vec3 {
    vec3(0.5, 0.7, 1.0)
}

pub struct Tracer {
    pixel_buf: Vec<Pixel>,
    random_seed: bool,
    subsampling: u8,
}

impl Tracer {
    pub fn new() -> Self {
        Tracer {
            pixel_buf: vec![],
            random_seed: true,
            subsampling: 4,
        }
    }

    pub fn trace_frame(
        &mut self,
        cam: &Cam,
        [w, h]: [u32; 2],
        scene: &Scene,
    ) -> &[Pixel] {
        let [w, h] = [w as usize, h as usize];
        self.resize_pixel_buf(w, h);
        let (screen_origin, screen_x_dir, screen_y_dir) =
            cam.screen_vecs(w as f32, h as f32);
        let cam_pos = cam.pos;
        let seed = if self.random_seed { rand::random() } else { 0 };
        self.pixel_buf
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, buf)| {
                let seed = SmallRng::seed_from_u64(seed + y as u64).next_u64();
                for x in 0..w {
                    let u = x as f32 / w as f32;
                    let v = y as f32 / h as f32;
                    let primary_ray = Ray {
                        origin: cam_pos,
                        dir: (screen_origin
                            + u * screen_x_dir
                            + v * screen_y_dir)
                            .normalize(),
                        bounces: MAX_BOUNCES,
                        throughput: Vec3::repeat(1.0),
                        rng: &mut SmallRng::seed_from_u64(seed + x as u64),
                    };
                    buf[x] = to_u8_triple(glm::min(
                        &trace(primary_ray, &scene),
                        1.0,
                    ));
                }
            });
        &self.pixel_buf
    }

    pub fn toggle_random_seed(&mut self) {
        self.random_seed = !self.random_seed;
    }

    pub fn increase_subsampling_denom(&mut self) {
        self.subsampling = self.subsampling.saturating_add(1);
    }

    pub fn decrease_subsampling_denom(&mut self) {
        self.subsampling = cmp::max(1, self.subsampling - 1)
    }

    pub fn subsampling(&self) -> u8 {
        self.subsampling
    }

    fn resize_pixel_buf(&mut self, w: usize, h: usize) {
        let n = w as usize * h as usize;
        // Instead of just `resize`ing, reserve an exact capacity first, to make
        // sure we don't allocate unnecessary space.
        let n_additional = n.saturating_sub(self.pixel_buf.len());
        self.pixel_buf.reserve_exact(n_additional);
        self.pixel_buf.resize(n, ERR_COLOR);
    }
}

fn trace(ray: Ray, scene: &[Sphere]) -> Vec3 {
    if let Some(hit) = closest_hit(&ray, scene) {
        let wo = -ray.dir;
        let hit_pos = ray.origin + hit.t * ray.dir;
        let radiance = direct_light(&hit, hit_pos, wo, scene);
        let sample = sample_wi(ray.rng, wo, hit.normal, hit.mat);
        let cosineterm = sample.wi.dot(&hit.normal).abs();
        // A probability of 0 means our sampled wi is actually impossible, and
        // the resulting BRDF won't make sense. Avoid nonsensical computations
        // (which will result in NaNs) by just setting throughput to 0.
        let throughput = if sample.pdf != 0.0 {
            ray.throughput
                .component_mul(&((sample.brdf * cosineterm) / sample.pdf))
        } else {
            Vec3::zeros()
        };
        let mut result = radiance.component_mul(&ray.throughput);
        if ray.bounces > 0 && glm::comp_max(&throughput) > 0.01 {
            let indirect_ray = Ray {
                origin: hit_pos + RAY_EPSILON * sample.wi,
                dir: sample.wi,
                bounces: ray.bounces - 1,
                throughput,
                ..ray
            };
            result += trace(indirect_ray, scene)
        }
        result
    } else {
        background_color().component_mul(&ray.throughput)
    }
}

fn direct_light(hit: &Hit, hit_pos: Vec3, wo: Vec3, scene: &[Sphere]) -> Vec3 {
    let light_pos = vec3(10.0, 20.0, -10.0);
    let light_emission = vec3(1.0, 0.95, 0.9) * 1_400.0;
    let dist = (light_pos - hit_pos).magnitude();
    let wl = (light_pos - hit_pos).normalize();
    // If surface and light aren't facing eachother at all, there can't be any
    // light contribution
    if hit.normal.dot(&wl) <= 0.0 {
        return Vec3::zeros();
    }
    let shadow_ray = BasicRay {
        origin: hit_pos + RAY_EPSILON * wl,
        dir: wl,
    };
    let in_shadow = any_hit(&shadow_ray, scene).is_some();
    if in_shadow {
        return Vec3::zeros();
    }
    // convert area based pdf to solid angle
    let weight = brdf(wl, wo, hit.normal, &hit.mat)
	// Optimal lighting conditions if the center point of both the light and
	// surface are exactly facing eachother
	* hit.normal.dot(&wl)
	// Falloff. Intensity drops proportionally to the square of the distance
        / (dist * dist);
    light_emission.component_mul(&weight)
}

fn to_u8_triple(v: Vec3) -> (u8, u8, u8) {
    (
        (v.x * 255.0) as u8,
        (v.y * 255.0) as u8,
        (v.z * 255.0) as u8,
    )
}
