pub mod shader;

use std::borrow::Borrow;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::import;
use crate::resources::shader::{Shader, ShaderCompiler, ShaderStage};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResourceId(String);

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResourceId {
    pub fn new<S: Into<String>>(s: S) -> ResourceId {
        let string = s.into();
        assert!(string.starts_with('/'));
        Self(string)
    }
}

#[derive(Clone)]
pub struct Resources {
    loader: FsDataLoader,
    shader_compiler: Arc<Mutex<ShaderCompiler>>,
}

impl Resources {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        Self {
            loader: FsDataLoader::new(root),
            shader_compiler: Arc::new(Mutex::new(ShaderCompiler::new())),
        }
    }

    pub fn load_binary<I: Borrow<ResourceId>>(&self, id: I) -> Result<Vec<u8>> {
        self.loader.load_binary(id)
    }

    pub fn load_model<I: Borrow<ResourceId>>(&self, id: I) -> Result<Model> {
        let data = self.loader.load_binary(id)?;

        import::obj(data)
    }

    pub fn load_shader<I: Borrow<ResourceId>>(&self, id: I, stage: ShaderStage) -> Result<Shader> {
        let path = self.loader.resolve_resource_id(id.borrow());
        let shader_compiler = self.shader_compiler.lock().unwrap();

        shader_compiler.compile_hlsl(&self.loader, path.to_str().unwrap(), stage)
    }
}

#[derive(Clone)]
pub struct FsDataLoader {
    root: PathBuf,
}

impl FsDataLoader {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        Self { root: root.into() }
    }

    fn resolve_resource_id(&self, id: &ResourceId) -> PathBuf {
        assert!(id.0.starts_with('/'), "resource ids must start with '/'");
        self.root.join(id.0.strip_prefix('/').unwrap())
    }

    pub fn load_binary<I: Borrow<ResourceId>>(&self, id: I) -> Result<Vec<u8>> {
        let real_path = self.resolve_resource_id(id.borrow());
        let data =
            std::fs::read(real_path).with_context(|| format!("unable to load {}", id.borrow()))?;

        Ok(data)
    }

    pub fn load_binary_from_raw_path(&self, path: &str) -> Result<Vec<u8>> {
        let data = std::fs::read(path).with_context(|| format!("unable to load {}", path))?;

        Ok(data)
    }
}

pub struct Model {
    meshes: Vec<Mesh>,
}

impl Model {
    pub fn new() -> Self {
        Self { meshes: Vec::new() }
    }

    pub fn add_mesh(&mut self, mesh: Mesh) {
        self.meshes.push(mesh);
    }
}

pub struct Mesh {
    data: Vec<f32>,
}

impl Mesh {
    pub fn new() -> Mesh {
        Mesh { data: Vec::new() }
    }

    pub fn add_vertex(&mut self, vertex: Vertex) {
        self.data.push(vertex.position.x);
        self.data.push(vertex.position.y);
        self.data.push(vertex.position.z);

        self.data.push(vertex.normal.x);
        self.data.push(vertex.normal.y);
        self.data.push(vertex.normal.z);

        self.data.push(vertex.texcoord.x);
        self.data.push(vertex.texcoord.y);
    }
}

pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub texcoord: Vec2,
}
