use std::path::PathBuf;
use std::sync::RwLock;

use ahash::AHashMap;
use uuid::Uuid;

use crate::shader::{Shader, ShaderCompiler, ShaderStage};

pub mod model;
pub mod shader;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct AssetId(Uuid);

impl AssetId {
    fn new() -> AssetId {
        AssetId(Uuid::new_v4())
    }
}

pub struct Vfs {
    shader_compiler: ShaderCompiler,
    root: PathBuf,

    name_id_map: RwLock<AHashMap<String, AssetId>>,
    id_name_map: RwLock<AHashMap<AssetId, String>>,
}

impl Vfs {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            shader_compiler: ShaderCompiler::new(),
            root: root.into(),

            name_id_map: RwLock::new(AHashMap::new()),
            id_name_map: RwLock::new(AHashMap::new()),
        }
    }

    fn real_path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    pub fn load_binary_sync(&self, path: &str) -> Vec<u8> {
        std::fs::read(self.real_path(path)).unwrap()
    }

    pub fn load_string_sync(&self, path: &str) -> String {
        std::fs::read_to_string(self.real_path(path)).unwrap()
    }

    pub fn load_shader_sync(&self, path: &str, stage: ShaderStage) -> Shader {
        let path = self.real_path(path);

        self.shader_compiler
            .compile_hlsl(path.to_str().unwrap(), stage)
            .unwrap()
    }

    pub fn acquire_asset_id_for_path(&self, path: &str) -> AssetId {
        let id = self.name_id_map.read().unwrap().get(path).cloned();

        if let Some(id) = id {
            return id;
        }

        let id = AssetId::new();

        self.name_id_map
            .write()
            .unwrap()
            .insert(path.to_owned(), id);

        self.id_name_map.write().unwrap().insert(id, path.to_owned());

        id
    }

    pub fn load_by_id(&self, id: AssetId) -> Vec<u8> {
        let path = self.id_name_map.read().unwrap().get(&id).cloned().unwrap();

        self.load_binary_sync(&path)
    }
}
