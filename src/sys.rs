pub use crate::core::clear_events;

use crate::core::{Res, ResMut};
use crate::render::PreparedUi;
use crate::render::{Extent2D, Renderer};
use crate::ui::Ui;
use winit::window::Window;

pub fn prepare_ui(window: Res<Window>, mut ui: ResMut<Ui>, mut prepared_ui: ResMut<PreparedUi>) {
    *prepared_ui = ui.finish_frame(&window);
    ui.begin_frame(&window);
}

pub fn render(
    window: Res<Window>,
    prepared_ui: Res<PreparedUi>,
    mut renderer: ResMut<Renderer>,
) {
    let window_size = window.inner_size();

    let extent = Extent2D {
        width: window_size.width,
        height: window_size.height,
    };

    renderer.render(
        /*g.current_scene().primary_camera().view_projection(extent.aspect_ratio()),*/
        Default::default(),
        &prepared_ui,
        extent,
    );
}
