use std::borrow::Borrow;
use std::collections::HashMap;

use glam::{vec3, Mat4, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::renderer;
use crate::resources::ResourceId;

pub const ROOT: Uuid = Uuid::nil();

pub struct Levels {
    levels: HashMap<Uuid, Level>,
}

impl Levels {
    pub fn new() -> Self {
        let mut levels = HashMap::new();
        levels.insert(ROOT, Level::new());

        Self { levels }
    }

    pub fn root(&self) -> &Level {
        self.levels.get(&ROOT).unwrap()
    }

    pub fn add_level(&mut self, uuid: Uuid, level: Level) {
        self.levels.insert(uuid, level);
    }
}

#[derive(Serialize, Deserialize)]
pub enum Object {
    Model(ResourceId),
}

#[derive(Serialize, Deserialize)]
pub struct Entity {
    transform: Mat4,
    object: Object,
}

impl Entity {
    pub fn new(object: Object) -> Self {
        Self {
            transform: Mat4::IDENTITY,
            object,
        }
    }

    pub fn with_transform(mut self, transform: Mat4) -> Self {
        self.transform = transform;
        self
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Camera {
    position: Vec3,

    // fixme: use quaternions
    pitch: f32,
    yaw: f32,
    roll: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: vec3(0.0, 0.0, 0.0),
            pitch: 0.0,
            yaw: 0.0,
            roll: 0.0,
        }
    }
}

impl<C: Borrow<Camera>> renderer::Camera for C {
    fn world_transform(&self) -> Mat4 {
        todo!()
    }
}

#[derive(Serialize, Deserialize)]
pub struct Level {
    entities: HashMap<Uuid, Entity>,
    camera: Camera,
}

impl Level {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            camera: Camera::new(),
        }
    }

    pub fn add(&mut self, entity: Entity) -> Uuid {
        let id = Uuid::new_v4();
        self.entities.insert(id, entity);
        id
    }

    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    pub fn load(data: &[u8]) -> Self {
        serde_json::from_slice(data).unwrap()
    }

    pub fn save(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap()
    }
}
