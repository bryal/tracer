use {
    emigui::{widgets::Label, Emigui},
    std::time,
};

pub const GUI_SCALE: f32 = 2.0;

pub struct Gui {
    fps_t: time::Instant,
    fps_n: u16,
    fps: f32,
    pub emigui: Emigui,
    pub dims: [f32; 2],
}

impl Gui {
    pub fn new() -> Self {
        Self {
            fps_t: time::Instant::now(),
            fps_n: 0,
            fps: 42.0,
            emigui: Emigui::new(GUI_SCALE),
            dims: [0.0, 0.0],
        }
    }

    pub fn update(&mut self, [w_px, h_px]: [u32; 2]) {
        self.fps_n += 1;
        let dt = self.fps_t.elapsed().as_secs_f32();
        if dt > 1.0 {
            self.fps = self.fps_n as f32 / dt;
            self.fps_t = time::Instant::now();
            self.fps_n = 0;
        }
        self.dims = [w_px as f32 / GUI_SCALE, h_px as f32 / GUI_SCALE];
        let raw_input = emigui::RawInput {
            screen_size: emigui::math::vec2(self.dims[0], self.dims[1]),
            pixels_per_point: GUI_SCALE,
            ..Default::default()
        };
        self.emigui.new_frame(raw_input);
        let mut region = self.emigui.whole_screen_region();
        region.add(emigui::label!("FPS: {:.2}", self.fps));
    }
}
