use std::any::Any;

use uuid::Uuid;
use videoland_ap::AssetId;
use videoland_core::scene::{Node, Ty};

pub struct Mesh {
    mesh_id: AssetId,
}

impl Mesh {
    pub fn new(mesh_id: AssetId) -> Self {
        Self { mesh_id }
    }
}

impl Ty for Mesh {
    fn ty() -> Uuid {
        Uuid::from_bytes([
            0xD9, 0x10, 0x41, 0xBA, 0x99, 0xEF, 0x46, 0x72, 0x82, 0x69, 0xCB, 0x7D, 0x19, 0xA7,
            0x90, 0xDD,
        ])
    }
}

impl Node for Mesh {
    fn as_any(&self) -> &dyn Any {
        todo!()
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        todo!()
    }

    fn name(&self) -> &str {
        todo!()
    }

    fn ty(&self) -> Uuid {
        <Self as Ty>::ty()
    }
}
