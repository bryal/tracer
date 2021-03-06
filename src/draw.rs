use {
    crate::cam::*,
    crate::geom::Scene,
    crate::gui::*,
    crate::trace::*,
    luminance::{
        context::GraphicsContext,
        pipeline::{BoundTexture, Pipeline, ShadingGate},
        pixel::{self, NormR8UI, RGB32F},
        render_state::RenderState,
        shader::program::{Program, Uniform},
        tess::{Mode, Tess, TessBuilder},
        texture::{self, Dim2, Texture},
    },
    luminance_derive::{Semantics, UniformInterface, Vertex},
    luminance_glutin::GlutinSurface,
};

const G_VS: &'static str = include_str!("gui-vert.glsl");
const G_FS: &'static str = include_str!("gui-frag.glsl");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Semantics)]
pub enum GSemantics {
    #[sem(name = "a_pos", repr = "[f32; 2]", wrapper = "GPosition")]
    Position,
    #[sem(name = "a_color", repr = "[u8; 4]", wrapper = "GColor")]
    Color,
    #[sem(name = "a_tc", repr = "[u16; 2]", wrapper = "GTc")]
    Tc,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Vertex)]
#[vertex(sem = "GSemantics")]
struct GVertex {
    pos: GPosition,
    #[vertex(normalized = "true")]
    color: GColor,
    tc: GTc,
}

#[derive(UniformInterface)]
struct GShaderInterface {
    u_screen_size: Uniform<[f32; 2]>,
    u_tex_size: Uniform<[f32; 2]>,
    u_sampler: Uniform<
        &'static BoundTexture<
            'static,
            texture::Flat,
            texture::Dim2,
            pixel::NormUnsigned,
        >,
    >,
}

const T_VS: &'static str = include_str!("vert.glsl");
const T_FS: &'static str = include_str!("frag.glsl");

#[derive(Clone, Copy, Debug, Eq, PartialEq, Semantics)]
pub enum TSemantics {
    #[sem(name = "pos", repr = "[f32; 2]", wrapper = "TPosition")]
    Position,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Vertex)]
#[vertex(sem = "TSemantics")]
struct TVertex {
    pos: TPosition,
}

#[derive(UniformInterface)]
struct TShaderInterface {
    tex: Uniform<
        &'static BoundTexture<'static, texture::Flat, Dim2, pixel::Floating>,
    >,
}

pub struct GuiProgram(Program<GSemantics, (), GShaderInterface>);

impl GuiProgram {
    pub fn create() -> Self {
        GuiProgram(
            Program::from_strings(None, G_VS, None, G_FS)
                .expect("gui program creation")
                .ignore_warnings(),
        )
    }

    pub fn draw<'a>(
        &'a self,
        surface: &mut GlutinSurface,
        gui: &'a mut Gui,
    ) -> impl FnOnce(&Pipeline, &mut ShadingGate<GlutinSurface>, RenderState) + 'a
    {
        gui.update(surface.size());
        let mesh = gui.emigui.paint();
        let vertices = mesh
            .vertices
            .iter()
            .map(|v| GVertex {
                pos: GPosition::new([v.pos.x, v.pos.y]),
                color: GColor::new([
                    v.color.r, v.color.g, v.color.b, v.color.a,
                ]),
                tc: GTc::new([v.uv.0, v.uv.1]),
            })
            .collect::<Vec<_>>();
        let indices =
            mesh.indices.iter().map(|i| *i as u16).collect::<Vec<_>>();
        let tess = TessBuilder::new(surface)
            .add_vertices(vertices)
            .set_indices(indices)
            .set_mode(Mode::Triangle)
            .build()
            .expect("gui build tess");
        let emigui_tex = gui.emigui.texture();
        let (tex_w, tex_h) = (emigui_tex.width, emigui_tex.height);
        let n_mipmaps = 0;
        let tex = Texture::<_, _, NormR8UI>::new(
            surface,
            [tex_w as u32, tex_h as u32],
            n_mipmaps,
            texture::Sampler::default(),
        )
        .expect("gui texture creation");
        tex.upload(texture::GenMipmaps::No, emigui_tex.pixels.as_slice())
            .expect("gui upload texture");
        move |pipeline, s_gate, render_st| {
            let bound_tex = pipeline.bind_texture(&tex);
            s_gate.shade(&self.0, |iface, mut r_gate| {
                iface.u_screen_size.update(gui.dims);
                iface.u_tex_size.update([tex_w as f32, tex_h as f32]);
                iface.u_sampler.update(&bound_tex);
                r_gate.render(&render_st, |mut t_gate| {
                    t_gate.render(&tess);
                });
            })
        }
    }
}

pub struct TracerProgram(Program<TSemantics, (), TShaderInterface>);

impl TracerProgram {
    pub fn create() -> Self {
        TracerProgram(
            Program::from_strings(None, T_VS, None, T_FS)
                .expect("tracer program creation")
                .ignore_warnings(),
        )
    }

    pub fn draw<'a>(
        &'a self,
        surface: &mut GlutinSurface,
        tracer: &mut Tracer,
        cam: &Cam,
        scene: &Scene,
    ) -> impl FnOnce(&Pipeline, &mut ShadingGate<GlutinSurface>, RenderState) + 'a
    {
        let tess = fullscreen_quad(surface);
        let [sw, sh] = surface.size();
        let dims = [
            sw / tracer.subsampling() as u32,
            sh / tracer.subsampling() as u32,
        ];
        let pixels = tracer.trace_frame(cam, dims, scene);
        let n_mipmaps = 0;
        let sampler = texture::Sampler {
            min_filter: texture::MinFilter::Nearest,
            mag_filter: texture::MagFilter::Nearest,
            ..Default::default()
        };
        let tex =
            Texture::<_, _, RGB32F>::new(surface, dims, n_mipmaps, sampler)
                .expect("luminance texture creation");
        tex.upload(texture::GenMipmaps::No, pixels).unwrap();
        move |pipeline, s_gate, render_st| {
            let bound_tex = pipeline.bind_texture(&tex);
            s_gate.shade(&self.0, |iface, mut r_gate| {
                iface.tex.update(&bound_tex);
                r_gate.render(&render_st, |mut t_gate| {
                    t_gate.render(&tess);
                });
            });
        }
    }
}

fn fullscreen_quad<S>(surface: &mut S) -> Tess
where
    S: GraphicsContext,
{
    let vertices = [
        TVertex::new(TPosition::new([-1.0, -1.0])),
        TVertex::new(TPosition::new([1.0, -1.0])),
        TVertex::new(TPosition::new([1.0, 1.0])),
        TVertex::new(TPosition::new([-1.0, 1.0])),
    ];
    TessBuilder::new(surface)
        .add_vertices(vertices)
        .set_mode(Mode::TriangleFan)
        .build()
        .unwrap()
}
