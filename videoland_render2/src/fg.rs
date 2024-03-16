use ahash::AHashMap;
use uuid::Uuid;
use videoland_rhi as rhi;

pub struct ResourceContainer {
    textures: AHashMap<Uuid, rhi::Texture>,
}

impl ResourceContainer {
    pub fn new() -> Self {
        Self {
            textures: AHashMap::new(),
        }
    }

    pub fn read(&self, id: Uuid) -> Uuid {
        id
    }
}
