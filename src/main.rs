mod trace;

use luminance::blending::{Equation, Factor};
use luminance::context::GraphicsContext as _;
use luminance::pipeline::BoundTexture;
use luminance::pixel::{NormRGB8UI, NormUnsigned};
use luminance::render_state::RenderState;
use luminance::shader::program::{Program, Uniform};
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance::texture::{Dim2, Flat, GenMipmaps, Sampler, Texture};
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_glfw::{
    Action, GlfwSurface, Key, Surface, WindowDim, WindowEvent, WindowOpt,
};

use trace::*;

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

#[derive(UniformInterface)]
struct ShaderInterface {
    tex: Uniform<&'static BoundTexture<'static, Flat, Dim2, NormUnsigned>>,
}

fn main() {
    // Surface to render to and get events from.
    let mut surface = GlfwSurface::new(
        WindowDim::Windowed(800, 800),
        "Tracer",
        WindowOpt::default(),
    )
    .expect("GLFW surface creation");
    let program = Program::<Semantics, (), ShaderInterface>::from_strings(
        None,
        VERT_SHADER_SRC,
        None,
        FRAG_SHADER_SRC,
    )
    .expect("program creation")
    .ignore_warnings();
    let tess = fullscreen_quad(&mut surface);
    let render_st = RenderState::default().set_blending((
        Equation::Additive,
        Factor::SrcAlpha,
        Factor::Zero,
    ));
    let mut back_buffer = surface.back_buffer().unwrap();
    let mut resize = false;
    let mut tracer = Tracer::new();
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
        let tex = trace_texture(&mut tracer, &mut surface);
        surface.pipeline_builder().pipeline(
            &back_buffer,
            clear,
            |pipeline, mut s_gate| {
                let bound_tex = pipeline.bind_texture(&tex);
                s_gate.shade(&program, |iface, mut r_gate| {
                    iface.tex.update(&bound_tex);
                    r_gate.render(render_st, |mut t_gate| {
                        t_gate.render(&tess);
                    });
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

fn trace_texture(
    tracer: &mut Tracer,
    surface: &mut GlfwSurface,
) -> Texture<Flat, Dim2, NormRGB8UI> {
    let [sw, sh] = surface.size();
    let sub = 4;
    let dims = [sw / sub, sh / sub];
    let pixels = tracer.trace_frame(dims);
    let n_mipmaps = 0;
    let tex = Texture::new(surface, dims, n_mipmaps, Sampler::default())
        .expect("luminance texture creation");
    tex.upload(GenMipmaps::No, pixels).unwrap();
    tex
}
