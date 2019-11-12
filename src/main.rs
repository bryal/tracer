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

// Vertex semantics. Those are needed to instruct the GPU how to select vertex’s
// attributes from the memory we fill at render time, in shaders. Acts as
// "protocol" between GPU's memory regions and shaders.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Semantics)]
pub enum Semantics {
    #[sem(name = "pos", repr = "[f32; 2]", wrapper = "VertexPosition")]
    Position,
    #[sem(name = "color", repr = "[u8; 3]", wrapper = "VertexColor")]
    Color,
}

// We derive the Vertex trait automatically and we associate to each field the
// semantics that must be used on the GPU. The proc-macro derive Vertex will
// make sure for us every field we use have a mapping to the type you specified
// as semantics.
//
// Currently, we need to use #[repr(C))] to ensure Rust is not going to move
// struct's fields around.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Vertex)]
#[vertex(sem = "Semantics")]
struct Vertex {
    pos: VertexPosition,
    #[vertex(normalized = "true")]
    rgb: VertexColor,
}

fn main() {
    // Surface to render to and get events from.
    let mut surface = GlfwSurface::new(
        WindowDim::Windowed(960, 540),
        "Hello, world!",
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
    panic!("\n\nThank you for playing Wing Commander!\n\n")
}

fn fullscreen_quad(surface: &mut GlfwSurface) -> Tess {
    let vertices: [Vertex; 4] = [
        // First triangle – an RGB one.
        Vertex::new(
            VertexPosition::new([-1.0, -1.0]),
            VertexColor::new([0, 255, 0]),
        ),
        Vertex::new(
            VertexPosition::new([1.0, -1.0]),
            VertexColor::new([0, 0, 255]),
        ),
        Vertex::new(
            VertexPosition::new([-1.0, 1.]),
            VertexColor::new([255, 0, 0]),
        ),
        Vertex::new(
            VertexPosition::new([1.0, 1.0]),
            VertexColor::new([255, 51, 255]),
        ),
    ];
    TessBuilder::new(surface)
        .add_vertices(vertices)
        .set_mode(Mode::TriangleStrip)
        .build()
        .unwrap()
}
