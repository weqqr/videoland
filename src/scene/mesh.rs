use std::any::Any;

use crate::asset::AssetId;
use crate::scene::{Node, Ty};
use uuid::{uuid, Uuid};

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
        uuid!("a91804d7-6727-4e66-805e-a977074a799a")
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
