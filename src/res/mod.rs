use std::io::Cursor;
use std::path::PathBuf;

use anyhow::{Context, Ok, Result};
use glam::{Vec2, Vec3};

#[derive(Clone)]
pub struct Resources {
    root: PathBuf,
}

impl Resources {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        Self { root: root.into() }
    }

    pub fn load_binary(&self, path: &str) -> Result<Vec<u8>> {
        let real_path = self.root.join(path);
        let data =
            std::fs::read(real_path).with_context(|| format!("unable to load {:?}", path))?;

        Ok(data)
    }

    pub fn load_model(&self, path: &str) -> Result<Model> {
        let data = self.load_binary(path)?;

        Model::from_obj(data)
    }
}

pub struct Model {
    meshes: Vec<Mesh>,
}

impl Model {
    pub fn new() -> Self {
        Self { meshes: Vec::new() }
    }

    fn from_obj(data: Vec<u8>) -> Result<Self> {
        let reader = Cursor::new(data);
        let obj = obj::ObjData::load_buf(reader)?;

        let mut model = Self::new();

        let vertex = |indices: obj::IndexTuple| Vertex {
            position: obj.position[indices.0].into(),
            normal: indices.2.map(|n| obj.normal[n]).unwrap_or([0.0; 3]).into(),
            texcoord: indices.1.map(|t| obj.texture[t]).unwrap_or([0.5; 2]).into(),
        };

        for group in obj.objects.iter().flat_map(|o| o.groups.iter()) {
            let mut mesh = Mesh::new();

            for poly in &group.polys {
                let base = poly.0[0];

                for i in 0..poly.0.len() - 2 {
                    mesh.add_vertex(vertex(base));
                    mesh.add_vertex(vertex(poly.0[i + 1]));
                    mesh.add_vertex(vertex(poly.0[i + 2]));
                }
            }

            model.add_mesh(mesh);
        }

        Ok(Self { meshes: Vec::new() })
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
        Mesh {
            data: Vec::new(),
        }
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
