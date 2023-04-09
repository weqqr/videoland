pub mod shader;

use anyhow::anyhow;
use std::borrow::Borrow;
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::import;

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
    root: PathBuf,
}

impl Resources {
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

    pub fn load_model<I: Borrow<ResourceId>>(&self, id: I) -> Result<Model> {
        let data = self.load_binary(id)?;

        import::obj(data)
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
