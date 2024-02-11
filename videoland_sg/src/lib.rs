use std::any::Any;

use glam::{Quat, Vec3};

#[derive(Clone, Copy, Default)]
pub struct NodeId {
    index: usize,
}

impl NodeId {
    fn new(index: usize) -> Self {
        Self { index }
    }
}

pub struct SceneGraph {
    nodes: Vec<Spatial>,
    free_indexes: Vec<usize>,
}

impl SceneGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            free_indexes: Vec::new(),
        }
    }

    pub fn add_node<I: Into<N>, N: Node + 'static>(&mut self, n: I) -> NodeId {
        let value = Spatial {
            inner: Some(Box::new(n.into())),
            ..Default::default()
        };

        if let Some(id) = self.free_indexes.pop() {
            self.nodes[id] = value;
            NodeId::new(id)
        } else {
            let id = self.nodes.len();
            self.nodes.push(value);
            NodeId::new(id)
        }
    }

    pub fn node(&self, handle: NodeId) -> &Spatial {
        self.nodes.get(handle.index).unwrap()
    }

    pub fn node_mut(&mut self, handle: NodeId) -> &mut Spatial {
        self.nodes.get_mut(handle.index).unwrap()
    }
}

pub trait Node {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn name(&self) -> &str;
}

#[derive(Default)]
pub struct Spatial {
    parent: NodeId,
    children: Vec<NodeId>,
    transform: Transform,
    visible: bool,
    enabled: bool,
    inner: Option<Box<dyn Node>>,
}

impl Spatial {
    pub fn new() -> Self {
        unimplemented!()
    }
}

impl std::ops::Deref for Spatial {
    type Target = Spatial;

    fn deref(&self) -> &Self::Target {
        self
    }
}

impl Node for Spatial {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn name(&self) -> &str {
        "self"
    }
}

pub struct MeshNode {/* mesh_id: ap::AssetId */}

impl MeshNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Default)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quat,
}
