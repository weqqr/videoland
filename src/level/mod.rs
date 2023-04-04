use std::collections::HashMap;

use glam::Mat4;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::resources::ResourceId;

pub struct RenderList {
    pub upload: Vec<ResourceId>,
    pub render: Vec<Uuid>,
}

pub struct Levels {
    levels: HashMap<Uuid, Level>,
}

impl Levels {
    pub fn new() -> Self {
        Self {
            levels: HashMap::new(),
        }
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

#[derive(Serialize, Deserialize)]
pub struct Level {
    entities: HashMap<Uuid, Entity>,
}

impl Level {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
        }
    }

    pub fn add(&mut self, entity: Entity) -> Uuid {
        let id = Uuid::new_v4();
        self.entities.insert(id, entity);
        id
    }

    pub fn load(data: &[u8]) -> Self {
        serde_json::from_slice(data).unwrap()
    }

    pub fn save(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).unwrap()
    }

    pub fn render(&self) -> RenderList {
        let upload = Vec::new();
        let mut render = Vec::new();

        for (id, entity) in &self.entities {
            match entity.object {
                Object::Model(_) => {},
            }
        }

        RenderList { upload, render }
    }
}
