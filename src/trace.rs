use std::time;

type Pixel = (u8, u8, u8);

pub const ERR_COLOR_F: (f32, f32, f32) = (1.0, 0.0, 1.0);
pub const ERR_COLOR: Pixel = (
    (ERR_COLOR_F.0 * 255.0) as u8,
    (ERR_COLOR_F.1 * 255.0) as u8,
    (ERR_COLOR_F.2 * 255.0) as u8,
);

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
        for y in 0..h {
            for x in 0..w {
                let uv = (x as f32 / w as f32, y as f32 / h as f32);
                let r = trace(uv, self.t0);
                self.pixel_buf[y * w + x] = (
                    (r[0] * 255.0) as u8,
                    (r[1] * 255.0) as u8,
                    (r[2] * 255.0) as u8,
                );
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

fn trace((_u, _v): (f32, f32), t0: time::Instant) -> [f32; 3] {
    let t = t0.elapsed().as_secs_f64();
    let x = t * 4.0;
    let r = x.sin() * 0.5 + 0.5;
    let g = (x * 1.3).sin() * 0.5 + 0.5;
    let b = (x * 1.7).sin() * 0.5 + 0.5;
    [r as f32, g as f32, b as f32]
}
