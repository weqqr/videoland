use glam::{vec3, Mat4, Quat, Vec3};

use crate::scene::Node;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Camera {
    pub position: Vec3,
    pub pitch: f32,
    pub yaw: f32,
    pub fov: f32,
}

impl Camera {
    pub fn new() -> Self {
        Camera {
            position: vec3(0.0, 0.0, -1.0),
            pitch: 0.0,
            yaw: 0.0,
            fov: 75.0,
        }
    }

    pub fn rotate(&mut self, delta_pitch: f32, delta_yaw: f32) {
        self.pitch -= delta_pitch;

        // super janky but I don't get disoriented
        let pitch_modulo = ((self.pitch % 360.0) + 360.0) % 360.0;
        let delta_yaw = if pitch_modulo > 90.0 && pitch_modulo < 270.0 {
            -delta_yaw
        } else {
            delta_yaw
        };

        self.yaw += delta_yaw;
    }

    fn rotation(&self) -> Quat {
        let rotation_x = Quat::from_rotation_x(self.pitch.to_radians());
        let rotation_y = Quat::from_rotation_y(-self.yaw.to_radians());

        rotation_y * rotation_x
    }

    pub fn forward_right(&self) -> (Vec3, Vec3) {
        let look = self.rotation().mul_vec3(Vec3::NEG_Z);
        let right = look.cross(Vec3::Y).normalize();

        (look, right)
    }

    pub fn view_projection(&self, aspect_ratio: f32) -> Mat4 {
        let projection = Mat4::perspective_rh(self.fov.to_radians(), aspect_ratio, 0.1, 2000.0);

        // world should rotate inversely to camera rotation
        let world_rotation = Mat4::from_quat(self.rotation().inverse());

        // world should be shifted away from the camera
        let world_translation = Mat4::from_translation(-self.position);

        projection * world_rotation * world_translation
    }
}

impl From<Camera> for Node {
    fn from(value: Camera) -> Node {
        Node::Camera(value)
    }
}
