mod draw;
mod geom;
mod gui;
mod intersect;
mod material;
mod trace;

use {
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
    'app: loop {
        let dt = t_prev.elapsed().as_secs_f32();
        t_prev = time::Instant::now();
        let actions = parse_events(&mut surface);
        if actions.exit {
            break 'app;
        }
            }
        }
        if actions.resize {
            // Simply ask another backbuffer at the right dimension (no
            // allocation / reallocation).
            back_buffer = surface.back_buffer().unwrap();
        }
        }
        let clear = [ERR_COLOR_F.0, ERR_COLOR_F.1, ERR_COLOR_F.2, 1.0];
        let scene = scene_1(t0);
        let tracer_painter =
            tracer_program.draw(&mut surface, &mut tracer, &scene);
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
        // println!("event: {:?}", event);
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

