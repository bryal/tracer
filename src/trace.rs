use nalgebra_glm::{vec3, Vec3};
use std::time;

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

    pub fn trace_frame(&mut self, [w, h]: [u32; 2]) -> &[Pixel] {
        let [w, h] = [w as usize, h as usize];
        self.resize_pixel_buf(w, h);
        let t = self.t0.elapsed().as_secs_f64();
        let cam_pos = vec3(t.sin() as f32 * 5.0, 3.0, -10.0);
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
        let scene = [
            Sphere {
                centre: vec3(0.0, 0.0, 0.0),
                radius: 1.0,
                color: vec3(1.0, 0.0, 0.0),
            },
            Sphere {
                centre: vec3(1.0, 0.0, 5.0),
                radius: 1.6,
                color: vec3(0.0, 1.0, 0.0),
            },
            Sphere {
                centre: vec3(3.0, 0.0, 0.0),
                radius: 1.2,
                color: vec3(0.0, 0.0, 1.0),
            },
            Sphere {
                centre: vec3(0.0, -101.0, 0.0),
                radius: 100.0,
                color: vec3(0.3, 0.3, 0.3),
            },
        ];
        for y in 0..h {
            for x in 0..w {
                let u = x as f32 / w as f32;
                let v = y as f32 / h as f32;
                let primary_ray = Ray {
                    origin: cam_pos,
                    dir: (screen_origin + u * screen_x_dir + v * screen_y_dir)
                        .normalize(),
                };
                self.pixel_buf[y * w + x] =
                    to_u8_triple(trace(primary_ray, &scene));
            }
        }
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

fn trace(ray: Ray, scene: &[Sphere]) -> Vec3 {
    let hits = scene.iter().flat_map(|obj| obj.intersect(&ray));
    if let Some(hit) =
        hits.min_by(|h1, h2| h1.t.partial_cmp(&h2.t).expect("sorting hits"))
    {
        // let hit_pos = ray.origin + hit.t * ray.dir;
        let from_sun = vec3(-6.0, -10.0, 4.0).normalize();
        let brightness = hit.normal.dot(&-from_sun).max(0.0);
        hit.color * brightness
    } else {
        background_color()
    }
}

struct Ray {
    origin: Vec3,
    dir: Vec3,
}

struct Hit {
    t: f32,
    normal: Vec3,
    color: Vec3,
}

struct Sphere {
    centre: Vec3,
    radius: f32,
    color: Vec3,
}

impl Sphere {
    fn intersect(&self, ray: &Ray) -> Option<Hit> {
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

fn to_u8_triple(v: Vec3) -> (u8, u8, u8) {
    (
        (v.x * 255.0) as u8,
        (v.y * 255.0) as u8,
        (v.z * 255.0) as u8,
    )
}
