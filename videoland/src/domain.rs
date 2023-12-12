use std::ops::Mul;

use derive_more::{Deref, From, Into};

use glam::{Mat4, Quat, Vec3};
use hecs::Entity;
use hecs_macros::{Bundle, Query};
use uuid::Uuid;

#[derive(Clone, From, Into, Deref)]
pub struct Name(pub String);

#[derive(Clone, Copy, From, Into, Deref)]
pub struct ModelId(pub Uuid);

#[derive(Clone, Copy, From, Into, Deref)]
pub struct MeshId(pub Uuid);

#[derive(Clone, Copy, From, Into)]
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

#[derive(Bundle)]
pub struct ModelBundle {
    pub transform: Transform,
    pub id: ModelId,
}

#[derive(Query)]
pub struct MeshQuery<'a> {
    pub transform: &'a Transform,
    pub id: &'a MeshId,
    pub name: &'a Name,
}

#[derive(Bundle)]
pub struct MeshBundle {
    pub transform: Transform,
    pub id: MeshId,
    pub name: Name,
}

#[derive(Clone)]
pub struct Parent {
    pub entity: Entity,
    pub relative_transform: Transform,
}

#[derive(Clone, Copy)]
pub struct Player;

#[derive(Clone, Copy)]
pub struct RigidBody {
    pub scene: Uuid,
}
