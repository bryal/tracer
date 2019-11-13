use nalgebra_glm::{vec3, Vec3};
use rayon::prelude::*;
use std::time;

use crate::geom::*;
use crate::intersect::*;

type Pixel = (u8, u8, u8);

pub const ERR_COLOR_F: (f32, f32, f32) = (1.0, 0.0, 1.0);
pub const ERR_COLOR: Pixel = (
    (ERR_COLOR_F.0 * 255.0) as u8,
    (ERR_COLOR_F.1 * 255.0) as u8,
    (ERR_COLOR_F.2 * 255.0) as u8,
);

fn background_color() -> Vec3 {
    vec3(0.0, 0.0, 0.0)
}

pub struct Tracer {
    t0: time::Instant,
    pixel_buf: Vec<Pixel>,
}

impl Tracer {
    pub fn new() -> Self {
        Tracer {
            t0: time::Instant::now(),
            pixel_buf: vec![],
        }
    }

    pub fn trace_frame(&mut self, [w, h]: [u32; 2], scene: &Scene) -> &[Pixel] {
        let [w, h] = [w as usize, h as usize];
        self.resize_pixel_buf(w, h);
        let t = self.t0.elapsed().as_secs_f64() / 20.0;
        let cam_pos = vec3(t.sin() as f32 * 16.0, 4.0, t.cos() as f32 * 16.0);
        let cam_target = vec3(0.0, 0.0, 0.0);
        let cam_dir = (cam_target - cam_pos).normalize();
        let world_up = Vec3::y();
        let cam_right = cam_dir.cross(&world_up).normalize();
        let cam_up = cam_right.cross(&cam_dir).normalize();
        let fov = (45.0f32).to_radians();
        let aspect_ratio = w as f32 / h as f32;
        let (screen_origin, screen_x_dir, screen_y_dir) = {
            let f = fov / 2.0;
            let a = cam_dir * f.cos();
            let b = cam_up * f.sin();
            let c = -cam_right * f.sin() * aspect_ratio;
            let o = a - c - b;
            (o, 2.0 * (a - b - o), 2.0 * (a - c - o))
        };
        self.pixel_buf
            .par_chunks_mut(w)
            .enumerate()
            .for_each(|(y, buf)| {
                for x in 0..w {
                    let u = x as f32 / w as f32;
                    let v = y as f32 / h as f32;
                    let primary_ray = Ray {
                        origin: cam_pos,
                        dir: (screen_origin
                            + u * screen_x_dir
                            + v * screen_y_dir)
                            .normalize(),
                    };
                    buf[x] = to_u8_triple(trace(&primary_ray, &scene));
                }
            });
        &self.pixel_buf
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

fn trace(ray: &Ray, scene: &[Sphere]) -> Vec3 {
    if let Some(hit) = closest_hit(ray, scene) {
        // let hit_pos = ray.origin + hit.t * ray.dir;
        let from_sun = vec3(-6.0, -10.0, 4.0).normalize();
        let brightness = hit.normal.dot(&-from_sun).max(0.0);
        hit.color * brightness
    } else {
        background_color()
    }
}

fn to_u8_triple(v: Vec3) -> (u8, u8, u8) {
    (
        (v.x * 255.0) as u8,
        (v.y * 255.0) as u8,
        (v.z * 255.0) as u8,
    )
}
