#[macro_use]
extern crate vulkano;
extern crate vulkano_shaders;
extern crate vulkano_win;
extern crate winit;

mod display;

use display::display;
use winit::{Event, WindowEvent};

fn main() {
    display(|ev, done, recreate_swapchain| match ev {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => *done = true,
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => *recreate_swapchain = true,
        _ => (),
    })
}
