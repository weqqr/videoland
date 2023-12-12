use egui::{pos2, Frame, PointerButton, Rect, Sense, Ui};

use crate::camera::ArcballCameraController;
use crate::editor::EditorData;

pub struct Viewport {
    pub camera_controller: ArcballCameraController,
}

impl Viewport {
    pub fn new() -> Self {
        let camera_controller = ArcballCameraController::new();

        Self { camera_controller }
    }

    pub fn ui(&mut self, ui: &mut Ui, data: &mut EditorData) {
        Frame::none().inner_margin(1.0).show(ui, |ui| {
            let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            // painter.image(
            //     data.renderer.color_buffer_texture_id,
            //     resp.rect,
            //     uv,
            //     Color32::WHITE,
            // );

            // data.renderer.resize_output_texture(PhysicalSize {
            //     width: resp.rect.width() as u32,
            //     height: resp.rect.height() as u32,
            // });

            let shift_pressed = ui.input(|i| i.modifiers.shift);

            if resp.dragged_by(PointerButton::Middle) && !shift_pressed {
                const SENSITIVITY: f32 = 0.7;

                let delta = resp.drag_delta();

                self.camera_controller.pitch += SENSITIVITY * delta.y;
                self.camera_controller.yaw += SENSITIVITY * delta.x;
            }

            if resp.dragged_by(PointerButton::Middle) && shift_pressed {
                let delta = resp.drag_delta();

                let camera = self.camera_controller.camera();
                let (forward, right) = camera.forward_right();
                let up = forward.cross(right).normalize();

                self.camera_controller.pivot += -up * delta.y;
                self.camera_controller.pivot += -right * delta.x;
            }

            let scroll_delta = ui.input(|i| i.scroll_delta.y);

            if resp.hovered() && scroll_delta != 0.0 {
                self.camera_controller.move_by(-scroll_delta / 100.0);
            }
        });
    }
}
