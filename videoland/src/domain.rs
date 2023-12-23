use std::ops::Mul;

use glam::{Mat4, Quat, Vec3};
use uuid::Uuid;

#[derive(Clone)]
pub struct Name(pub String);

#[derive(Clone, Copy)]
pub struct ModelId(pub Uuid);

#[derive(Clone, Copy)]
pub struct MeshId(pub Uuid);

#[derive(Clone, Copy)]
pub struct RenderableMesh(pub Uuid);

#[derive(Clone, Copy)]
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


#[derive(Clone, Copy)]
pub struct Player;

#[derive(Clone, Copy)]
pub struct RigidBody {
    pub scene: Uuid,
}
