use crate::scene::{Camera, Mesh};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Node2 {
    Pivot,
    Mesh(Mesh),
    Camera(Camera),
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
