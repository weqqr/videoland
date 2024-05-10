use crate::core::ArenaHandle;
use crate::scene::{Camera, Mesh, Pivot, Spatial};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Node {
    Pivot(Pivot),
    Mesh(Mesh),
    Camera(Camera),
}

impl Node {
    pub fn pivot(&self) -> &Pivot {
        match self {
            Node::Pivot(pivot) => pivot,
            _ => panic!("node is not pivot"),
        }
    }

    pub fn mesh(&self) -> &Mesh {
        match self {
            Node::Mesh(mesh) => mesh,
            _ => panic!("node is not mesh"),
        }
    }

    pub fn camera(&self) -> &Camera {
        match self {
            Node::Camera(camera) => camera,
            _ => panic!("node is not camera"),
        }
    }
}

pub type NodeHandle = ArenaHandle<Spatial>;
