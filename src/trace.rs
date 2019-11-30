use nalgebra_glm as glm;
use nalgebra_glm::{vec3, Vec3};
use rand::prelude::*;
use rayon::prelude::*;
use std::cmp;

use crate::cam::*;
use crate::geom::*;
use crate::intersect::*;
use crate::material::*;

type Pixel = (f32, f32, f32);

const RAY_EPSILON: f32 = 0.0001;
const MAX_BOUNCES: u8 = 3;

pub const ERR_COLOR: (f32, f32, f32) = (1_000_000.0, 0.0, 1_000_000.0);

fn background_color() -> Vec3 {
    vec3(0.5, 0.7, 1.0)
}

pub struct Tracer {
    pixel_buf: Vec<Pixel>,
    random_seed: bool,
    subsampling: u8,
    accum_n_max: u64,
    accum_n: u64,
    reset_on_move: bool,
    dims: [u32; 2],
    prev_cam: Cam,
}

impl Tracer {
    pub fn new() -> Self {
        Tracer {
            pixel_buf: vec![],
            random_seed: true,
            subsampling: 4,
            accum_n_max: 0,
            accum_n: 0,
            reset_on_move: false,
            dims: [0, 0],
            prev_cam: Cam::new(Vec3::zeros(), Vec3::zeros()),
        }
    }

    pub fn trace_frame(
        &mut self,
        cam: &Cam,
        dims: [u32; 2],
        scene: &Scene,
    ) -> &[Pixel] {
        if self.accum_n_max == 0
            || (self.reset_on_move && cam != &self.prev_cam)
        {
            self.reset_accum()
        }
        self.prev_cam = cam.clone();
        if dims != self.dims {
            self.resize_pixel_buf(dims)
        }
        let [w, h] = [dims[0] as usize, dims[1] as usize];
        let (screen_origin, screen_x_dir, screen_y_dir) =
            cam.screen_vecs(w as f32, h as f32);
        let cam_pos = cam.pos;
        let seed = if self.random_seed {
            rand::random()
        } else {
            self.accum_n
        };
        let a = 1.0 / (self.accum_n + 1) as f32;
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
                    let color = trace(primary_ray, &scene);
                    let old_color = from_triple(buf[x]);
                    buf[x] = to_triple(glm::lerp(&old_color, &color, a));
                }
            });
        if self.accum_n < self.accum_n_max {
            self.accum_n += 1
        }
        &self.pixel_buf
    }

    pub fn toggle_random_seed(&mut self) {
        self.random_seed = !self.random_seed;
        self.reset_accum()
    }

    pub fn toggle_reset_on_move(&mut self) {
        self.reset_on_move = !self.reset_on_move;
        self.reset_accum()
    }

    pub fn toggle_accum(&mut self) {
        self.accum_n_max = if self.accum_n_max == 0 {
            std::u64::MAX
        } else {
            0
        };
        self.reset_accum()
    }

    pub fn decrease_accum_n_max(&mut self) {
        self.accum_n_max = self.accum_n_max.saturating_sub(1);
        self.reset_accum()
    }

    pub fn increase_accum_n_max(&mut self) {
        self.accum_n_max = self.accum_n_max.saturating_add(1);
        self.reset_accum()
    }

    pub fn decrease_subsampling_denom(&mut self) {
        self.subsampling = cmp::max(1, self.subsampling - 1);
        self.reset_accum()
    }

    pub fn increase_subsampling_denom(&mut self) {
        self.subsampling = self.subsampling.saturating_add(1);
        self.reset_accum()
    }

    pub fn reset_accum(&mut self) {
        self.accum_n = 0;
    }

    pub fn subsampling(&self) -> u8 {
        self.subsampling
    }

    fn resize_pixel_buf(&mut self, dims: [u32; 2]) {
        let n = dims[0] as usize * dims[1] as usize;
        // Instead of just `resize`ing, reserve an exact capacity first, to
        // make sure we don't allocate unnecessary space.
        let n_additional = n.saturating_sub(self.pixel_buf.len());
        self.pixel_buf.reserve_exact(n_additional);
        self.pixel_buf.resize(n, ERR_COLOR);
        self.dims = dims;
        self.reset_accum()
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

fn to_triple(v: Vec3) -> (f32, f32, f32) {
    (v.x, v.y, v.z)
}

fn from_triple((r, g, b): (f32, f32, f32)) -> Vec3 {
    vec3(r, g, b)
}
