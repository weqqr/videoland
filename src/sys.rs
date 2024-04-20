pub use crate::core::clear_events;

use crate::core::{Res, ResMut};
use crate::render::PreparedUi;
use crate::render::{Extent2D, Renderer};
use crate::scene::SceneGraph;
use crate::ui::Ui;
use winit::window::Window;

pub fn prepare_ui(window: Res<Window>, mut ui: ResMut<Ui>, mut prepared_ui: ResMut<PreparedUi>) {
    *prepared_ui = ui.finish_frame(&window);
    ui.begin_frame(&window);
}

pub fn update_transform_hierarchy(
    mut sg: ResMut<SceneGraph>,
) {
    for (_, scene) in sg.scenes_mut() {
        scene.update_transform_hierarchy();
    }
}

pub fn render_primary_scene(
    window: Res<Window>,
    prepared_ui: Res<PreparedUi>,
    mut renderer: ResMut<Renderer>,
    sg: Res<SceneGraph>,
) {
    let window_size = window.inner_size();

    let extent = Extent2D {
        width: window_size.width,
        height: window_size.height,
    };

    renderer.render(
        sg.current_scene()
            .primary_camera()
            .camera()
            .view_projection(extent.aspect_ratio()),
        sg.current_scene(),
        &prepared_ui,
        extent,
    );
}
