use nalgebra_glm::{vec2, vec3, U8Vec3, Vec2, Vec3};

type Pixel = (u8, u8, u8);

pub const ERR_COLOR_F: (f32, f32, f32) = (1.0, 0.0, 1.0);
pub const ERR_COLOR: Pixel = (
    (ERR_COLOR_F.0 * 255.0) as u8,
    (ERR_COLOR_F.1 * 255.0) as u8,
    (ERR_COLOR_F.2 * 255.0) as u8,
);

pub struct Tracer {
    pixel_buf: Vec<Pixel>,
}

impl Tracer {
    pub fn new() -> Self {
        Tracer { pixel_buf: vec![] }
    }

    pub fn trace_frame(&mut self, [w, h]: [u32; 2]) -> &[Pixel] {
        let [w, h] = [w as usize, h as usize];
        self.resize_pixel_buf(w, h);
        for y in 0..h {
            for x in 0..w {
                let uv = vec2(x as f32 / w as f32, y as f32 / h as f32);
                self.pixel_buf[y * w + x] = to_triple(to_u8vec3(trace(uv)));
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

fn trace(uv: Vec2) -> Vec3 {
    let r = uv.x;
    let g = (uv.x * 3.7 + uv.y * 5.1).sin() * 0.5 + 0.5;
    let b = 1.0 - uv.y;
    vec3(r, g, b)
}

fn to_triple<T, V: Into<[T; 3]>>(v: V) -> (T, T, T) {
    let [x, y, z] = v.into();
    (x, y, z)
}

fn to_u8vec3(v: Vec3) -> U8Vec3 {
    v.map(|x| (x * 255.0) as u8)
}
