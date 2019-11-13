mod geom;
mod intersect;
mod trace;

use luminance::context::GraphicsContext as _;
use luminance::pipeline::BoundTexture;
use luminance::pixel::{NormRGB8UI, NormUnsigned};
use luminance::render_state::RenderState;
use luminance::shader::program::{Program, Uniform};
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance::texture::{self, Dim2, Texture};
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_glfw::{
    Action, GlfwSurface, Key, Surface, WindowDim, WindowEvent, WindowOpt,
};
use std::time;

use geom::*;
use trace::*;

const SUBSAMPLING: u32 = 8;

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
    tex: Uniform<
        &'static BoundTexture<'static, texture::Flat, Dim2, NormUnsigned>,
    >,
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
    let render_st = RenderState::default();
    let mut back_buffer = surface.back_buffer().unwrap();
    let mut resize = false;
    let mut tracer = Tracer::new();
    let mut t = time::Instant::now();
    let mut tf = 0.0;
    let mut nf = 0;
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
        let scene = build_scene(t0);
        let tex = trace_texture(&mut tracer, &mut surface, &scene);
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
        let t_prev = t;
        t = time::Instant::now();
        let dt = t_prev.elapsed().as_secs_f64();
        tf += dt;
        nf += 1;
        if tf > 1.0 {
            println!("FPS: {:.3}", nf as f64 / tf);
            tf = 0.0;
            nf = 0;
        }
    }
    // Something is not always dropping correctly, probably an Arc somewhere, so
    // we do this to force exit.
    println!("\nThank you for playing Wing Commander!\n");
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
    scene: &Scene,
) -> Texture<texture::Flat, Dim2, NormRGB8UI> {
    let [sw, sh] = surface.size();
    let dims = [sw / SUBSAMPLING, sh / SUBSAMPLING];
    let pixels = tracer.trace_frame(dims, scene);
    let n_mipmaps = 0;
    let sampler = texture::Sampler {
        min_filter: texture::MinFilter::Nearest,
        mag_filter: texture::MagFilter::Nearest,
        ..Default::default()
    };
    let tex = Texture::new(surface, dims, n_mipmaps, sampler)
        .expect("luminance texture creation");
    tex.upload(texture::GenMipmaps::No, pixels).unwrap();
    tex
}
