use luminance::context::GraphicsContext as _;
use luminance::render_state::RenderState;
use luminance::shader::program::Program;
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance_derive::{Semantics, Vertex};
use luminance_glfw::{
    Action, GlfwSurface, Key, Surface, WindowDim, WindowEvent, WindowOpt,
};

const VERT_SHADER_SRC: &'static str = include_str!("vert.glsl");
const FRAG_SHADER_SRC: &'static str = include_str!("frag.glsl");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Semantics)]
pub enum Semantics {
    #[sem(name = "pos", repr = "[f32; 2]", wrapper = "VertexPosition")]
    Position,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Vertex)]
#[vertex(sem = "Semantics")]
struct Vertex {
    pos: VertexPosition,
}

fn main() {
    // Surface to render to and get events from.
    let mut surface = GlfwSurface::new(
        WindowDim::Windowed(800, 800),
        "Tracer",
        WindowOpt::default(),
    )
    .expect("GLFW surface creation");
    let program = Program::<Semantics, (), ()>::from_strings(
        None,
        VERT_SHADER_SRC,
        None,
        FRAG_SHADER_SRC,
    )
    .expect("program creation")
    .ignore_warnings();
    let tesselation = fullscreen_quad(&mut surface);
    let mut back_buffer = surface.back_buffer().unwrap();
    let mut resize = false;
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
            surface.back_buffer().unwrap();
            resize = false;
        }
        let clear = [0.0, 0.0, 0.0, 0.0];
        surface.pipeline_builder().pipeline(
            &back_buffer,
            clear,
            |_, mut shading_gate| {
                shading_gate.shade(&program, |_, mut render_gate| {
                    render_gate.render(
                        RenderState::default(),
                        |mut tess_gate| {
                            tess_gate.render(&tesselation);
                        },
                    );
                });
            },
        );
        surface.swap_buffers();
    }
    // Something is not always dropping correctly, probably an Arc somewhere, so
    // we do this to force exit.
    println!("\nThank you for playing Wing Commander!");
    std::process::abort();
}

fn fullscreen_quad(surface: &mut GlfwSurface) -> Tess {
    let vertices: [Vertex; 4] = [
        Vertex::new(VertexPosition::new([-1.0, -1.0])),
        Vertex::new(VertexPosition::new([1.0, -1.0])),
        Vertex::new(VertexPosition::new([1.0, 1.0])),
        Vertex::new(VertexPosition::new([-1.0, 1.0])),
    ];
    TessBuilder::new(surface)
        .add_vertices(vertices)
        .set_mode(Mode::TriangleFan)
        .build()
        .unwrap()
}
