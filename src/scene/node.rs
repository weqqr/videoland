use std::any::Any;

use uuid::Uuid;

pub trait Node: 'static {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn name(&self) -> &str;
    fn ty(&self) -> Uuid;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

pub trait NodeType {
    fn node_type() -> Uuid;
}
