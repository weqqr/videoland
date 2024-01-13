pub use videoland_ecs::clear_events;

use videoland_ecs::{Events, Res, ResMut};
use videoland_render2::egui::PreparedUi;
use videoland_render2::{Extent2D, Renderer};
use winit::event::KeyEvent;
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

use crate::camera::Camera;
use crate::ui::Ui;

pub fn prepare_ui(window: Res<Window>, mut ui: ResMut<Ui>, mut prepared_ui: ResMut<PreparedUi>) {
    *prepared_ui = ui.finish_frame(&window);
    ui.begin_frame(&window);
}

pub fn show_test_window(ui: Res<Ui>) {
    egui::Window::new("--videoland-test-window").show(ui.ctx(), |ui| {
        ui.label("Hello, world!");
    });
}

pub fn render(window: Res<Window>, prepared_ui: Res<PreparedUi>, mut renderer: ResMut<Renderer>) {
    let window_size = window.inner_size();

    let extent = Extent2D {
        width: window_size.width,
        height: window_size.height,
    };

    let camera = &Camera::new();

    renderer.render(extent, &prepared_ui);
}
