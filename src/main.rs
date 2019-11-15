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
    luminance_glfw::{
        Action, GlfwSurface, Key, Surface, WindowDim, WindowEvent, WindowOpt,
    },
    std::time,
    trace::*,
};

fn main() {
    // Surface to render to and get events from.
    let mut surface = GlfwSurface::new(
        WindowDim::Windowed(800, 800),
        "Tracer",
        WindowOpt::default(),
    )
    .expect("GLFW surface creation");
    let tracer_program = draw::TracerProgram::create();
    let gui_program = draw::GuiProgram::create();
    let render_st = RenderState::default().set_blending((
        blending::Equation::Additive,
        blending::Factor::SrcAlpha,
        blending::Factor::SrcAlphaComplement,
    ));
    let mut back_buffer = surface.back_buffer().unwrap();
    let mut resize = false;
    let mut tracer = Tracer::new();
    let mut gui = Gui::new();
    let t0 = time::Instant::now();
    'app: loop {
        for event in surface.poll_events() {
            match event {
                WindowEvent::Close
                | WindowEvent::Key(Key::Escape, _, Action::Release, _) => break 'app,
                WindowEvent::FramebufferSize(..) => {
                    resize = true;
                }
                _ => (),
            }
        }
        if resize {
            // Simply ask another backbuffer at the right dimension (no
            // allocation / reallocation).
            back_buffer = surface.back_buffer().unwrap();
            resize = false;
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
