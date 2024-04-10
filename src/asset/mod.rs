use std::path::PathBuf;
use std::sync::RwLock;

use ahash::AHashMap;
use uuid::Uuid;

mod model;
mod scene;
mod shader;

pub use self::model::*;
pub use self::scene::*;
pub use self::shader::*;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct AssetId(Uuid);

impl AssetId {
    fn new() -> AssetId {
        AssetId(Uuid::new_v4())
    }
}

pub struct Vfs {
    roots: RwLock<AHashMap<String, PathBuf>>,

    name_id_map: RwLock<AHashMap<String, AssetId>>,
    id_name_map: RwLock<AHashMap<AssetId, String>>,
}

impl Vfs {
    pub fn new() -> Self {
        Self {
            roots: RwLock::new(AHashMap::new()),

            name_id_map: RwLock::new(AHashMap::new()),
            id_name_map: RwLock::new(AHashMap::new()),
        }
    }

    pub fn add_root(&self, name: String, path: impl Into<PathBuf>) {
        self.roots.write().unwrap().insert(name, path.into());
    }

    fn real_path(&self, path: &str) -> PathBuf {
        let root_name = content_root_for_path(dbg!(path)).unwrap();
        let root = self.roots.read().unwrap();

        let relative_path = path
            .strip_prefix('/')
            .and_then(|path| path.strip_prefix(root_name))
            .and_then(|path| path.strip_prefix('/'))
            .unwrap();

        root.get(root_name).unwrap().join(relative_path)
    }

    pub fn load_binary_sync(&self, path: &str) -> Vec<u8> {
        std::fs::read(self.real_path(path)).unwrap()
    }

    pub fn load_string_sync(&self, path: &str) -> String {
        std::fs::read_to_string(self.real_path(path)).unwrap()
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

        self.id_name_map
            .write()
            .unwrap()
            .insert(id, path.to_owned());

        id
    }

    pub fn load_by_id(&self, id: AssetId) -> Vec<u8> {
        let path = self.id_name_map.read().unwrap().get(&id).cloned().unwrap();

        self.load_binary_sync(&path)
    }
}

fn content_root_for_path(path: &str) -> Option<&str> {
    path.strip_prefix('/')?.split('/').next()
}
