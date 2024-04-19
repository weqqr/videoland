use crate::scene::{Camera, Mesh, Pivot};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct NodeId {
    pub(super) index: usize,
}

impl NodeId {
    pub const NONE: NodeId = NodeId { index: usize::MAX };
}

impl Default for NodeId {
    fn default() -> Self {
        Self::NONE
    }
}

impl NodeId {
    pub(super) fn new(index: usize) -> Self {
        Self { index }
    }
}
