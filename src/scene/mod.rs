use std::ops::{Deref, DerefMut};

mod camera;
mod mesh;
mod node;
mod transform;

pub use self::camera::*;
pub use self::mesh::*;
pub use self::node::*;
pub use self::transform::*;

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

    pub fn add_node(&mut self, node: Spatial) -> NodeId {
        if let Some(id) = self.free_indexes.pop() {
            self.nodes[id] = node;
            NodeId::new(id)
        } else {
            let id = self.nodes.len();
            self.nodes.push(node);
            NodeId::new(id)
        }
    }

    pub fn spatial(&self, handle: NodeId) -> &Spatial {
        self.nodes.get(handle.index).unwrap()
    }

    pub fn node<T: Node>(&self, handle: NodeId) -> NodeRef<T> {
        self.spatial(handle).node()
    }

    pub fn spatial_mut(&mut self, handle: NodeId) -> &mut Spatial {
        self.nodes.get_mut(handle.index).unwrap()
    }

    pub fn node_mut<T: Node>(&mut self, handle: NodeId) -> NodeRefMut<T> {
        self.spatial_mut(handle).node_mut()
    }
}

pub struct Spatial {
    parent: NodeId,
    children: Vec<NodeId>,
    transform: Transform,
    visible: bool,
    enabled: bool,
    node: Option<Box<dyn Node>>,
}

impl Spatial {
    pub fn new<N: Node>(node: N) -> Self {
        Self {
            node: Some(Box::new(node)),
            ..Self::empty()
        }
    }

    pub fn empty() -> Self {
        Self {
            parent: NodeId::NONE,
            children: Vec::new(),
            transform: Transform::default(),
            visible: true,
            enabled: true,
            node: None,
        }
    }

    pub fn node<N: Node>(&self) -> NodeRef<N> {
        NodeRef {
            parent: &self.parent,
            children: &self.children,
            transform: &self.transform,
            visible: &self.visible,
            enabled: &self.enabled,
            node: self
                .node
                .as_ref()
                .map(|x| x.as_any().downcast_ref().unwrap()),
        }
    }

    pub fn node_mut<N: Node>(&mut self) -> NodeRefMut<N> {
        NodeRefMut {
            parent: &mut self.parent,
            children: &mut self.children,
            transform: &mut self.transform,
            visible: &mut self.visible,
            enabled: &mut self.enabled,
            node: self
                .node
                .as_mut()
                .map(|x| x.as_any_mut().downcast_mut().unwrap()),
        }
    }

    pub fn with_parent(mut self, parent: NodeId) -> Self {
        self.parent = parent;
        self
    }

    pub fn with_children(mut self, parent: NodeId) -> Self {
        self.parent = parent;
        self
    }

    pub fn with_transform(mut self, transform: Transform) -> Self {
        self.transform = transform;
        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn attach_child(&mut self, child: NodeId) {
        assert_ne!(child, NodeId::NONE, "attached node ID must not be NONE");

        self.children.push(child);
    }

    pub fn detach_child(&mut self, child: NodeId) {
        let Some(position) = self.children.iter().position(|c| *c == child) else {
            return;
        };

        self.children.remove(position);
    }
}

pub struct NodeRef<'a, N: Node> {
    pub parent: &'a NodeId,
    pub children: &'a Vec<NodeId>,
    pub transform: &'a Transform,
    pub visible: &'a bool,
    pub enabled: &'a bool,
    pub node: Option<&'a N>,
}

impl<'a, N: Node> Deref for NodeRef<'a, N> {
    type Target = N;

    fn deref(&self) -> &Self::Target {
        self.node.unwrap()
    }
}

pub struct NodeRefMut<'a, N: Node> {
    pub parent: &'a mut NodeId,
    pub children: &'a mut Vec<NodeId>,
    pub transform: &'a mut Transform,
    pub visible: &'a mut bool,
    pub enabled: &'a mut bool,
    pub node: Option<&'a mut N>,
}

impl<'a, N: Node> Deref for NodeRefMut<'a, N> {
    type Target = N;

    fn deref(&self) -> &Self::Target {
        self.node.as_ref().unwrap()
    }
}

impl<'a, N: Node> DerefMut for NodeRefMut<'a, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.node.as_mut().unwrap()
    }
}
