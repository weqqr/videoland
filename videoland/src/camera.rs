use glam::{vec3, Mat4, Quat, Vec3};

pub struct MainCamera {
    pub camera: Camera,
}

impl MainCamera {
    pub fn new() -> Self {
        Self {
            camera: Camera::new(),
        }
    }
}

#[derive(Clone)]
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

        // Vertical axis needs to be inverted when using Vulkan. Engine assumes
        // positive Y to go up, and Vulkan does the opposite.
        //
        // +---------------------+
        // |  Engine  |  Vulkan  |
        // +----------+----------+
        // | Y ^      |        X |
        // |   |      |   0----> |
        // |   0----> |   |      |
        // |        X | Y v      |
        // +----------+----------+
        //
        let flip = Mat4::from_scale(vec3(1.0, -1.0, 1.0));

        flip * projection * world_rotation * world_translation
    }
}

pub struct ArcballCameraController {
    pub pivot: Vec3,
    pub pitch: f32,
    pub yaw: f32,
    pub step: f32,
}

impl ArcballCameraController {
    pub fn new() -> Self {
        Self {
            pivot: Vec3::ZERO,
            pitch: 0.0,
            yaw: 0.0,
            step: 20.0,
        }
    }

    pub fn camera(&self) -> Camera {
        let rotation_x = Quat::from_rotation_x(self.pitch.to_radians());
        let rotation_y = Quat::from_rotation_y(-self.yaw.to_radians());

        let distance = 1.3f32.powf(self.step);
        let position = (rotation_y * rotation_x).mul_vec3(Vec3::NEG_Z * distance) + self.pivot;

        Camera {
            position,
            pitch: -self.pitch,
            yaw: self.yaw + 180.0,
            fov: 90.0f32,
        }
    }

    pub fn rotate_by(&mut self, delta_pitch: f32, delta_yaw: f32) {
        self.pitch += delta_pitch;
        self.yaw += delta_yaw;
    }

    pub fn move_by(&mut self, delta_step: f32) {
        self.step += delta_step;
    }
}
