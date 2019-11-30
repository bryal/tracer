mod cam;
mod draw;
mod geom;
mod gui;
mod intersect;
mod material;
mod trace;

use {
    cam::*,
    geom::*,
    gui::Gui,
    luminance::{
        blending, context::GraphicsContext as _, render_state::RenderState,
    },
    luminance_glutin::{
        CursorMode, ElementState::*, Event, GlutinSurface,
        KeyboardInput as KeyInp, LogicalPosition, Surface,
        VirtualKeyCode as Key, WindowDim, WindowEvent, WindowOpt,
    },
    nalgebra_glm::{vec2, vec3, Vec2, Vec3},
    std::collections::HashSet,
    std::time,
    trace::*,
};

const MOVE_SPEED: f32 = 8.0;

fn main() {
    // Surface to render to and get events from.
    let mut surface = GlutinSurface::new(
        WindowDim::Windowed(800, 800),
        "Tracer",
        WindowOpt::default().set_cursor_mode(CursorMode::Disabled),
    )
    .expect("Glutin surface creation");
    let tracer_program = draw::TracerProgram::create();
    let gui_program = draw::GuiProgram::create();
    let render_st = RenderState::default().set_blending((
        blending::Equation::Additive,
        blending::Factor::SrcAlpha,
        blending::Factor::SrcAlphaComplement,
    ));
    let mut back_buffer = surface.back_buffer().unwrap();
    let mut tracer = Tracer::new();
    let mut gui = Gui::new();
    let t0 = time::Instant::now();
    let mut t_prev = time::Instant::now();
    let scenes = [scene_0, scene_1, scene_2];
    let mut scene_i = 0;
    let mut cam = Cam::new(vec3(0.0, 4.0, 16.0), Vec3::zeros());
    let mut input_st = InputState::new(&mut surface);
    'app: loop {
        let dt = t_prev.elapsed().as_secs_f32();
        t_prev = time::Instant::now();
        let actions = parse_events(&mut surface);
        if actions.exit {
            break 'app;
        }
        if let Some(pos) = actions.cursor {
            let dp = input_st.cursor_pos(&mut surface, pos);
            if dp.magnitude() > 0.001 {
                cam.mouse_rotate(dp)
            }
        }
        if actions.resize {
            // Simply ask another backbuffer at the right dimension (no
            // allocation / reallocation).
            back_buffer = surface.back_buffer().unwrap();
            reset_cursor_pos(&mut surface);
        }
        input_st.press_all(actions.presseds);
        input_st.release_all(actions.releaseds);
        if input_st.pressed(Key::Z) {
            scene_i = (scene_i + 1) % scenes.len();
        }
        if input_st.pressed(Key::R) {
            tracer.toggle_random_seed()
        }
        if input_st.pressed(Key::Period) {
            tracer.increase_subsampling_denom()
        }
        if input_st.pressed(Key::Comma) {
            tracer.decrease_subsampling_denom()
        }
        let move_d = dt * MOVE_SPEED;
        if input_st.held(Key::W) {
            cam.move_forwards(move_d)
        }
        if input_st.held(Key::S) {
            cam.move_backwards(move_d)
        }
        if input_st.held(Key::D) {
            cam.move_right(move_d)
        }
        if input_st.held(Key::A) {
            cam.move_left(move_d)
        }
        if input_st.held(Key::Space) {
            cam.move_up(move_d)
        }
        if input_st.held(Key::LShift) {
            cam.move_down(move_d)
        }
        let clear = [ERR_COLOR.0, ERR_COLOR.1, ERR_COLOR.2, 1.0];
        let scene = scenes[scene_i](t0);
        let tracer_painter =
            tracer_program.draw(&mut surface, &mut tracer, &cam, &scene);
        let gui_painter = gui_program.draw(&mut surface, &mut gui);
        surface.pipeline_builder().pipeline(
            &back_buffer,
            clear,
            |pipeline, mut s_gate| {
                tracer_painter(&pipeline, &mut s_gate, render_st);
                gui_painter(&pipeline, &mut s_gate, render_st);
            },
        );
        surface.swap_buffers();
    }
    // Something is not always dropping correctly, probably an Arc somewhere, so
    // we do this to force exit.
    println!("\nThank you for playing Wing Commander!\n");
    std::process::abort();
}

struct Actions {
    exit: bool,
    resize: bool,
    cursor: Option<Vec2>,
    presseds: HashSet<Key>,
    releaseds: HashSet<Key>,
}

fn parse_events(surface: &mut GlutinSurface) -> Actions {
    let mut actions = Actions {
        exit: false,
        resize: false,
        cursor: None,
        presseds: HashSet::new(),
        releaseds: HashSet::new(),
    };
    for event in surface.poll_events() {
        match event {
            Event::WindowEvent { event, .. } => {
                parse_window_event(event, &mut actions)
            }
            _ => (),
        }
    }
    actions
}

/// Returns whether we should exit
fn parse_window_event(e: WindowEvent, actions: &mut Actions) {
    match e {
        WindowEvent::CloseRequested => actions.exit = true,
        WindowEvent::KeyboardInput {
            input:
                KeyInp {
                    state,
                    virtual_keycode: Some(k),
                    ..
                },
            ..
        } => {
            if k == Key::Escape {
                actions.exit = true;
            } else if state == Pressed {
                actions.presseds.insert(k);
            } else {
                actions.releaseds.insert(k);
            }
        }
        WindowEvent::CursorMoved { position, .. } => {
            actions.cursor = Some(vec2(position.x as f32, position.y as f32))
        }
        WindowEvent::Resized(_) => actions.resize = true,
        _ => (),
    }
}

struct InputState {
    pressed_keys: HashSet<Key>,
    held_keys: HashSet<Key>,
    released_keys: HashSet<Key>,
}

impl InputState {
    fn new(surface: &mut GlutinSurface) -> Self {
        reset_cursor_pos(surface);
        Self {
            pressed_keys: HashSet::new(),
            held_keys: HashSet::new(),
            released_keys: HashSet::new(),
        }
    }

    fn press_all(&mut self, ks: HashSet<Key>) {
        self.held_keys.extend(&ks);
        self.pressed_keys = ks;
    }

    fn release_all(&mut self, ks: HashSet<Key>) {
        for k in &ks {
            self.held_keys.remove(k);
        }
        self.released_keys = ks
    }

    fn pressed(&self, k: Key) -> bool {
        self.pressed_keys.contains(&k)
    }

    fn held(&self, k: Key) -> bool {
        self.held_keys.contains(&k)
    }

    /// Returns normalized position difference
    fn cursor_pos(
        &mut self,
        surface: &mut GlutinSurface,
        logical_pos: Vec2,
    ) -> Vec2 {
        reset_cursor_pos(surface);
        let [w, h] = surface.size();
        let pos_normalized =
            logical_pos.component_div(&vec2(w as f32, h as f32));
        let midpoint_normalized = vec2(0.5, 0.5);
        pos_normalized - midpoint_normalized
    }
}

fn reset_cursor_pos(surface: &mut GlutinSurface) {
    let [w, h] = surface.size();
    surface.set_cursor_position(LogicalPosition {
        x: w as f64 / 2.0,
        y: h as f64 / 2.0,
    });
}
