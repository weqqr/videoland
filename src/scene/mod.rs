use std::ops::{Deref, DerefMut};

mod camera;
mod mesh;
mod node;
mod transform;

use slab::Slab;

pub use self::camera::*;
pub use self::mesh::*;
pub use self::node::*;
pub use self::transform::*;

pub struct SceneGraph {
    nodes: Vec<Scene>,
    primary_scene_id: Option<SceneId>,
}

impl SceneGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            primary_scene_id: None,
        }
    }

    pub fn add_scene(&mut self, scene: Scene) -> SceneId {
        let id = self.nodes.len();
        self.nodes.push(scene);
        SceneId::new(id)
    }

    pub fn set_primary_scene_id(&mut self, id: SceneId) {
        self.primary_scene_id = Some(id);
    }

    pub fn primary_scene_id(&self) -> SceneId {
        self.primary_scene_id.expect("primary scene not set")
    }

    pub fn scene(&self, id: SceneId) -> Option<&Scene> {
        self.nodes.get(id.index)
    }

    pub fn scenes(&self) -> impl Iterator<Item = (SceneId, &Scene)> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(id, scene)| (SceneId::new(id), scene))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneId {
    pub(super) index: usize,
}

impl SceneId {
    fn new(index: usize) -> Self {
        Self { index }
    }
}

pub struct Scene {
    pub bg_color: u32,
    nodes: Slab<Spatial>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            bg_color: 0x102030FF,
            nodes: Slab::new(),
        }
    }

    pub fn add_node(&mut self, node: Spatial) -> NodeId {
        NodeId::new(self.nodes.insert(node))
    }

    pub fn nodes(&self) -> impl Iterator<Item = (NodeId, &Spatial)> {
        self.nodes
            .iter()
            .map(|(id, spatial)| (NodeId::new(id), spatial))
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

#[derive(Clone, serde::Serialize, serde::Deserialize)]
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
