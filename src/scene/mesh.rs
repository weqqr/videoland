use crate::asset::AssetId;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Mesh {
    mesh_id: AssetId,
}

impl Mesh {
    pub fn new(mesh_id: AssetId) -> Self {
        Self { mesh_id }
    }
}
