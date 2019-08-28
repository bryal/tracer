use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, DynamicState};
use vulkano::descriptor::PipelineLayoutAbstract;
use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::format::ClearValue;
use vulkano::framebuffer::{Framebuffer, FramebufferAbstract, RenderPassAbstract, Subpass};
use vulkano::image::SwapchainImage;
use vulkano::instance::{Instance, PhysicalDevice, QueueFamily};
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::swapchain;
use vulkano::swapchain::{
    AcquireError, PresentMode, Surface, SurfaceTransform, Swapchain, SwapchainCreationError,
};
use vulkano::sync;
use vulkano::sync::{FlushError, GpuFuture};
use winit::{Event, EventsLoop, Window};

use std::iter;
use std::sync::Arc;

#[derive(Default, Debug, Clone)]
struct Vertex {
    pos: [f32; 2],
}

impl_vertex!(Vertex, pos);

mod vert {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
#version 450
layout(location = 0) in vec2 pos;
void main() {
    gl_Position = vec4(pos, 0.0, 1.0);
}
        "
    }
}

mod frag {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
#version 450
layout(location = 0) out vec4 f_color;
void main() {
    f_color = vec4(0.0, 1.0, 0.0, 1.0);
}
        "
    }
}

pub fn display<F>(f: F)
where
    F: Fn(Event, &mut bool, &mut bool),
{
    // Initialization

    let instance = create_vulkan_instance();
    let physical_device = get_physical_device(&instance);
    let (mut window_events_loop, surface) = create_surface(instance.clone());
    let window = surface.window();
    let queue_family = find_drawing_queue_family(&physical_device, &surface);
    let (device, queue) = initialize_device(physical_device, queue_family);
    let (mut swapchain, images) = create_swapchain(physical_device, &surface, &device, &queue);
    let render_pass = create_render_pass(&device, &swapchain);
    let (vert, frag) = load_shaders(&device);
    let pipeline = create_pipeline(vert, frag, &render_pass, &device);
    // Dynamic viewports allow us to recreate just the viewport when the window is resized
    let mut dynamic_state = DynamicState {
        line_width: None,
        viewports: None,
        scissors: None,
    };
    let mut framebuffers =
        window_size_dependent_setup(&images, render_pass.clone(), &mut dynamic_state);

    // Initialization finished!

    // The swapchain may become invalid for various reason. We may have to recreate it.
    let mut recreate_swapchain = false;
    // Submitting a command to the GPU produces a `GpuFuture`, which
    // holds the resources for as long as they are in use by the GPU.
    //
    // Destroying the `GpuFuture` blocks until the GPU is finished executing it. In order to avoid
    // that, we store the submission of the previous frame here.
    let mut previous_frame_end = Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>;

    loop {
        // It is important to call this function from time to time, otherwise resources will keep
        // accumulating and you will eventually reach an out of memory error.
        // Calling this function polls various fences in order to determine what the GPU has
        // already processed, and frees the resources that are no longer needed.
        previous_frame_end.cleanup_finished();
        // Whenever the window resizes we need to recreate everything dependent on the window size
        if recreate_swapchain {
            let dimensions = window_dimensions(window);
            let (new_swapchain, new_images) = match swapchain.recreate_with_dimension(dimensions) {
                Ok(r) => r,
                // This error tends to happen when the user is manually resizing the window.
                // Simply restarting the loop is the easiest way to fix this issue.
                Err(SwapchainCreationError::UnsupportedDimensions) => continue,
                Err(err) => panic!("{:?}", err),
            };
            swapchain = new_swapchain;
            // Because framebuffers contains an Arc on the old swapchain, we need to
            // recreate framebuffers as well.
            framebuffers =
                window_size_dependent_setup(&new_images, render_pass.clone(), &mut dynamic_state);
            recreate_swapchain = false;
        }
        // Acquire an image from the swapchain. Blocks with optional
        // timeout until image is available. Returns the index of the
        // image that we are allowed to draw upon.
        let timeout = None;
        let (image_num, acquire_future) =
            match swapchain::acquire_next_image(swapchain.clone(), timeout) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    recreate_swapchain = true;
                    continue;
                }
                Err(err) => panic!("{:?}", err),
            };
        let vertex_buffer = fullscreen_quad(&device);
        // We're rendering a full-screen quad every frame anyways, so clearing is pointless.
        let clear_values = vec![ClearValue::None];
        // Build a command buffer. Holds the list of commands that are going to be executed.
        //
        // Note that we have to pass a queue family when we create the command buffer. The command
        // buffer will only be executable on that given queue family.
        let command_buffer =
            AutoCommandBufferBuilder::primary_one_time_submit(device.clone(), queue.family())
                .unwrap()
                // Enter a render pass. There are two methods to do
                // this: `draw_inline` and `draw_secondary`. The latter is a bit more advanced.
                .begin_render_pass(framebuffers[image_num].clone(), false, clear_values)
                .unwrap()
                // We are now inside the first subpass of the render pass. We add a draw command.
                //
                // The last two parameters contain the list of resources to pass to the shaders.
                // Since we used an `EmptyPipeline` object, the objects have to be `()`.
                .draw(
                    pipeline.clone(),
                    &dynamic_state,
                    vertex_buffer.clone(),
                    (),
                    (),
                )
                .unwrap()
                // We leave the render pass by calling `draw_end`. Note that if we had multiple
                // subpasses we could have called `next_inline` (or `next_secondary`) to jump to the
                // next subpass.
                .end_render_pass()
                .unwrap()
                .build()
                .unwrap();
        let future = previous_frame_end
            .join(acquire_future)
            .then_execute(queue.clone(), command_buffer)
            .unwrap()
            // The color output is now expected to contain our
            // triangle. Show the image on the screen by calling `present`.
            .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
            .then_signal_fence_and_flush();
        match future {
            Ok(future) => {
                previous_frame_end = Box::new(future) as Box<_>;
            }
            Err(FlushError::OutOfDate) => {
                recreate_swapchain = true;
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            }
            Err(e) => {
                println!("{:?}", e);
                previous_frame_end = Box::new(sync::now(device.clone())) as Box<_>;
            }
        }
        // Handle the window events
        let mut done = false;
        window_events_loop.poll_events(|ev| f(ev, &mut done, &mut recreate_swapchain));
        if done {
            return;
        }
    }
}

fn create_vulkan_instance() -> Arc<Instance> {
    let app_info = app_info_from_cargo_toml!();
    let extensions = vulkano_win::required_extensions();
    // NOTE: This could be used for FPS counters and other debug stuff.
    let layers = None;
    Instance::new(Some(&app_info), &extensions, layers).expect("Failed to create vulkan instance")
}

fn get_physical_device<'a>(instance: &'a Arc<Instance>) -> PhysicalDevice<'a> {
    println!("Available physical devices:");
    for dev in PhysicalDevice::enumerate(&instance) {
        println!("    {}", dev.name());
    }
    // TODO: Filter out unsupported devices.
    // TODO: Let user choose device
    let physical = PhysicalDevice::from_index(&instance, 0).expect("Device 0 out of range");
    println!(
        "Using device: {} (type: {:?})",
        physical.name(),
        physical.ty()
    );
    physical
}

fn create_surface(instance: Arc<Instance>) -> (EventsLoop, Arc<Surface<Window>>) {
    let events = EventsLoop::new();
    let builder = winit::WindowBuilder::new();
    let surface = vulkano_win::VkSurfaceBuild::build_vk_surface(builder, &events, instance)
        .expect("Failed to create window / vulkan surface");
    (events, surface)
}

/// Choose which GPU queue will execute out draw commands.
///
/// "A queue family is group of one or multiple queues. All queues of
/// one family have the same characteristics."
fn find_drawing_queue_family<'a>(
    dev: &'a PhysicalDevice,
    surf: &Surface<Window>,
) -> QueueFamily<'a> {
    dev.queue_families()
        .find(|&q| q.supports_graphics() && surf.is_supported(q).unwrap_or(false))
        .expect("Could not find any supported queue family with drawing capabilities graphics")
}

fn initialize_device(
    physical: PhysicalDevice,
    queue_family: vulkano::instance::QueueFamily,
) -> (Arc<Device>, Arc<Queue>) {
    // Some parts of the Vulkan specs are optional and must be enabled
    // manually at device creation. In this example the only thing we
    // are going to need is the `khr_swapchain` extension that allows
    // us to draw to a window.
    let device_ext = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };
    // The floating-point represents the priority of the queue between
    // 0.0 and 1.0. The priority of the queue is a hint to the
    // implementation about how much it should prioritize queues
    // between one another.
    let queue_families = iter::once((queue_family, 0.5));
    let (device, mut queues) = Device::new(
        physical,
        physical.supported_features(),
        &device_ext,
        queue_families,
    )
    .expect("Failed to initialize device");
    // Since we can request multiple queues, the `queues` variable is in fact an iterator. In this
    // example we use only one queue, so we just retrieve the first and only element of the
    // iterator and throw it away.
    let queue = queues.next().unwrap();
    (device, queue)
}

/// Creating a swapchain allocates the color buffers that will contain
/// the image that will ultimately be visible on the screen. These
/// images are returned alongside with the swapchain.
fn create_swapchain(
    physical: PhysicalDevice,
    surface: &Arc<Surface<Window>>,
    device: &Arc<Device>,
    queue: &Arc<Queue>,
) -> (Arc<Swapchain<Window>>, Vec<Arc<SwapchainImage<Window>>>) {
    let window = surface.window();
    let caps = surface.capabilities(physical).unwrap();
    let usage = caps.supported_usage_flags;
    // The alpha mode indicates how the alpha value of the final image will behave. For example
    // you can choose whether the window will be opaque or transparent.
    let alpha = caps.supported_composite_alpha.iter().next().unwrap();
    // Choosing the internal format that the images will have.
    let format = caps.supported_formats[0].0;
    let initial_dimensions = window_dimensions(window);
    Swapchain::new(
        device.clone(),
        surface.clone(),
        caps.min_image_count,
        format,
        initial_dimensions,
        1,
        usage,
        queue,
        SurfaceTransform::Identity,
        alpha,
        PresentMode::Fifo,
        true,
        None,
    )
    .expect("Failed to create swapchain")
}

fn window_dimensions(window: &Window) -> [u32; 2] {
    if let Some(dimensions) = window.get_inner_size() {
        // convert to physical pixels
        let dimensions: (u32, u32) = dimensions.to_physical(window.get_hidpi_factor()).into();
        [dimensions.0, dimensions.1]
    } else {
        // The window no longer exists so exit the application.
        panic!("The window is gone! Exiting...")
    }
}

fn create_render_pass(
    device: &Arc<Device>,
    swapchain: &Swapchain<Window>,
) -> Arc<impl RenderPassAbstract> {
    let pass = single_pass_renderpass!(
        device.clone(),
        attachments: {
            // `color` is a custom name we give to the first and only attachment.
            color: {
                // `load: Clear` means that we ask the GPU to clear the content of this attachment
                // at the start of the drawing. `DontCare` means we'll draw over it anyways, so do
                // whatever.
                load: DontCare,
                // `store: Store` means that we ask the GPU to store the output of the draw
                // in the actual image. We could also ask it to discard the result.
                store: Store,
                // `format: <ty>` indicates the type of the format of the image. This has to
                // be one of the types of the `vulkano::format` module (or alternatively one
                // of your structs that implements the `FormatDesc` trait). Here we use the
                // same format as the swapchain.
                format: swapchain.format(),
                samples: 1,
            }
        },
        pass: {
            // We use the attachment named `color` as the one and only color attachment.
            color: [color],
            // No depth-stencil attachment is indicated with empty brackets.
            depth_stencil: {}
        }
    );
    Arc::new(pass.unwrap())
}

fn load_shaders(device: &Arc<Device>) -> (vert::Shader, frag::Shader) {
    let vert = vert::Shader::load(device.clone()).expect("Failed to load vertex shader");
    let frag = frag::Shader::load(device.clone()).expect("Failed to load fragment shader");
    (vert, frag)
}

fn create_pipeline<R>(
    vert: vert::Shader,
    frag: frag::Shader,
    render_pass: &Arc<R>,
    device: &Arc<Device>,
) -> Arc<
    GraphicsPipeline<
        SingleBufferDefinition<Vertex>,
        Box<dyn PipelineLayoutAbstract + Send + Sync>,
        Arc<R>,
    >,
>
where
    R: RenderPassAbstract,
{
    let pipeline = GraphicsPipeline::start()
        .vertex_input_single_buffer::<Vertex>()
        .vertex_shader(vert.main_entry_point(), ())
        .triangle_strip()
        // Use a resizable viewport set to draw over the entire window
        .viewports_dynamic_scissors_irrelevant(1)
        .fragment_shader(frag.main_entry_point(), ())
        // The pipeline will only be usable from this particular subpass
        .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
        .build(device.clone())
        .unwrap();
    Arc::new(pipeline)
}

/// This method is called once during initialization, then again whenever the window is resized
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
    dynamic_state: &mut DynamicState,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions();
    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };
    dynamic_state.viewports = Some(vec![viewport]);
    images
        .iter()
        .map(|image| {
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(image.clone())
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>()
}

fn fullscreen_quad(device: &Arc<Device>) -> Arc<CpuAccessibleBuffer<[Vertex]>> {
    CpuAccessibleBuffer::from_iter(
        device.clone(),
        BufferUsage::all(),
        [
            Vertex { pos: [-1.0, -1.0] },
            Vertex { pos: [1.0, -1.0] },
            Vertex { pos: [-1.0, 1.0] },
            Vertex { pos: [1.0, 1.0] },
        ]
        .iter()
        .cloned(),
    )
    .unwrap()
}
