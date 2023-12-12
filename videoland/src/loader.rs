use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use hecs::{Entity, World};
use rayon::ThreadPool;
use uuid::Uuid;

use crate::geometry::Model;
use crate::render2::Shader;
use crate::shader_compiler::{ShaderCompiler, ShaderStage};

pub struct Loader {
    shader_compiler: ShaderCompiler,
    thread_pool: Arc<ThreadPool>,

    root: PathBuf,

    models_pending_attachment_to_entity: DashMap<Entity, Model, ahash::RandomState>,
}

pub struct LoadedAsset {
    id: Uuid,
}

impl Loader {
    pub fn new(root: PathBuf, thread_pool: Arc<ThreadPool>) -> Self {
        let shader_compiler = ShaderCompiler::new();

        Self {
            shader_compiler,
            thread_pool,

            root,

            models_pending_attachment_to_entity: DashMap::with_hasher(ahash::RandomState::new()),
        }
    }

    fn real_path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    pub fn load_binary(&self, path: &str) -> Vec<u8> {
        std::fs::read(self.real_path(path)).unwrap()
    }

    pub fn load_string(&self, path: &str) -> String {
        std::fs::read_to_string(self.real_path(path)).unwrap()
    }

    pub fn load_shader(&self, path: &str, stage: ShaderStage) -> Shader {
        let path = self.real_path(path);

        self.shader_compiler
            .compile_hlsl(path.to_str().unwrap(), stage)
            .unwrap()
    }

    pub fn load_and_attach_model_sync(&self, e: Entity, path: &str) {
        let data = self.load_binary(path);
        let model = Model::from_obj(&data);

        self.models_pending_attachment_to_entity.insert(e, model);
    }

    pub fn poll(&mut self, world: &mut World) {
        let ready_entities: Vec<_> = self
            .models_pending_attachment_to_entity
            .iter()
            .map(|refm| *refm.pair().0)
            .collect();

        for id in ready_entities {
            let (_, model) = self
                .models_pending_attachment_to_entity
                .remove(&id)
                .unwrap();

            world.insert_one(id, model).unwrap();
        }
    }
}
