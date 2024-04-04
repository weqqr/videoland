use std::ops::Mul;

use glam::{Mat4, Quat, Vec3};

#[derive(Default, Clone, Copy)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.position)
    }
}

impl Mul for Transform {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            position: self.position + rhs.position,
            rotation: self.rotation * rhs.rotation,
        }
    }
}
