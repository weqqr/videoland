pub use videoland_core::clear_events;

use videoland_core::{Res, ResMut};
use videoland_egui::Ui;
use videoland_render2::egui::PreparedUi;
use videoland_render2::{Extent2D, Renderer};
use winit::window::Window;

use crate::camera::MainCamera;

pub fn prepare_ui(window: Res<Window>, mut ui: ResMut<Ui>, mut prepared_ui: ResMut<PreparedUi>) {
    *prepared_ui = ui.finish_frame(&window);
    ui.begin_frame(&window);
}

pub fn render(
    window: Res<Window>,
    camera: Res<MainCamera>,
    prepared_ui: Res<PreparedUi>,
    mut renderer: ResMut<Renderer>,
) {
    let window_size = window.inner_size();

    let extent = Extent2D {
        width: window_size.width,
        height: window_size.height,
    };

    renderer.render(camera.camera.view_projection(extent.aspect_ratio()), &prepared_ui, extent);
}
