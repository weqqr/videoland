use crate::asset::AssetId;
use crate::scene::Node;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Mesh {
    mesh_id: AssetId,
}

impl Mesh {
    pub fn new(mesh_id: AssetId) -> Self {
        Self { mesh_id }
    }
}

impl From<Mesh> for Node {
    fn from(value: Mesh) -> Node {
        Node::Mesh(value)
    }
}
