use std::io::Cursor;

use glam::{Vec2, Vec3};
use uuid::Uuid;

pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub texcoord: Vec2,
}

impl Vertex {
    fn write(&self, data: &mut Vec<f32>) {
        data.extend_from_slice(&self.position.to_array());
        data.extend_from_slice(&self.normal.to_array());
        data.extend_from_slice(&self.texcoord.to_array());
    }

    pub fn layout() -> ra::VertexBufferLayout<'static> {
        ra::VertexBufferLayout {
            stride: 8 * 4,
            attributes: &[
                // position
                ra::VertexAttribute {
                    binding: 0,
                    format: ra::VertexFormat::Float32x3,
                    offset: 0,
                    location: 0,
                },
                // normal
                ra::VertexAttribute {
                    binding: 0,
                    format: ra::VertexFormat::Float32x3,
                    offset: 3 * 4,
                    location: 1,
                },
                // texcoord
                ra::VertexAttribute {
                    binding: 0,
                    format: ra::VertexFormat::Float32x2,
                    offset: 6 * 4,
                    location: 2,
                },
            ],
        }
    }
}

pub struct Mesh {
    pub id: Uuid,
    pub name: String,
    vertex_count: u32,
    data: Vec<f32>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            vertex_count: 0,
            data: Vec::new(),
        }
    }

    pub fn add_vertex(&mut self, vertex: Vertex) {
        self.vertex_count += 1;
        vertex.write(&mut self.data);
    }

    pub fn vertex_count(&self) -> u32 {
        self.vertex_count
    }

    pub fn data(&self) -> &[f32] {
        &self.data
    }
}

pub struct Model {
    pub id: Uuid,
    pub name: String,
    meshes: Vec<Mesh>,
}

impl Model {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::new(),
            meshes: Vec::new(),
        }
    }

    pub fn add_mesh(&mut self, mesh: Mesh) {
        self.meshes.push(mesh);
    }

    pub fn meshes(&self) -> impl Iterator<Item = &Mesh> {
        self.meshes.iter()
    }

    pub fn from_obj(data: &[u8]) -> Self {
        let reader = Cursor::new(data);
        let obj = obj::ObjData::load_buf(reader).unwrap();

        let mut model = Model::new();

        let vertex = |indices: obj::IndexTuple| Vertex {
            position: obj.position[indices.0].into(),
            normal: indices.2.map(|n| obj.normal[n]).unwrap_or([0.0; 3]).into(),
            texcoord: indices.1.map(|t| obj.texture[t]).unwrap_or([0.5; 2]).into(),
        };

        for group in obj.objects.iter().flat_map(|o| o.groups.iter()) {
            let mut mesh = Mesh::new();
            mesh.name = group.name.clone();

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

        model
    }
}
