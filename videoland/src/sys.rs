pub use videoland_ecs::clear_events;

use videoland_ecs::{Res, ResMut, Events};
use videoland_render2::{Renderer, Extent2D};
use winit::event::KeyEvent;
use winit::keyboard::{NamedKey, Key};
use winit::window::Window;

use crate::camera::Camera;

pub fn render(window: Res<Window>, mut renderer: ResMut<Renderer>) {
    let window_size = window.inner_size();

    let extent = Extent2D {
        width: window_size.width,
        height: window_size.height,
    };

    let camera = &Camera::new();

    renderer.render(extent);
}

pub fn handle_input(input: Events<KeyEvent>) {
    for key in input.iter() {
        if let Key::Named(NamedKey::Escape) = key.logical_key {
            println!("escape");
        }
    }
}
