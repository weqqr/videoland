use std::ops::{Deref, DerefMut};

mod camera;
mod mesh;
mod node;
mod pivot;
mod transform;

use crate::core::{Arena, ArenaHandle};

pub use self::camera::*;
pub use self::mesh::*;
pub use self::node::*;
pub use self::pivot::*;
pub use self::transform::*;

pub struct SceneGraph {
    nodes: Arena<Scene>,
    current_scene_id: Option<SceneHandle>,
}

impl SceneGraph {
    pub fn new() -> Self {
        Self {
            nodes: Arena::new(),
            current_scene_id: None,
        }
    }

    pub fn add_scene(&mut self, scene: Scene) -> SceneHandle {
        self.nodes.insert(scene)
    }

    pub fn set_current_scene_id(&mut self, id: SceneHandle) {
        self.current_scene_id = Some(id);
    }

    pub fn current_scene_id(&self) -> SceneHandle {
        self.current_scene_id.expect("current scene not set")
    }

    pub fn current_scene(&self) -> &Scene {
        self.scene(self.current_scene_id())
            .expect("current scene doesn't exist")
    }

    pub fn scene(&self, id: SceneHandle) -> Option<&Scene> {
        self.nodes.get(id)
    }

    pub fn scenes(&self) -> impl Iterator<Item = (SceneHandle, &Scene)> {
        self.nodes.iter()
    }

    pub fn scenes_mut(&mut self) -> impl Iterator<Item = (SceneHandle, &mut Scene)> {
        self.nodes.iter_mut()
    }
}

pub type SceneHandle = ArenaHandle<Scene>;

pub struct Scene {
    pub bg_color: u32,
    primary_camera_id: Option<NodeHandle>,
    nodes: Arena<Spatial>,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            bg_color: 0x102030FF,
            primary_camera_id: None,
            nodes: Arena::new(),
        }
    }

    pub fn update_transform_hierarchy(&mut self) {}

    pub fn add_node(&mut self, node: Spatial) -> NodeHandle {
        self.nodes.insert(node)
    }

    pub fn set_primary_camera_id(&mut self, id: NodeHandle) {
        self.primary_camera_id = Some(id);
    }

    pub fn primary_camera(&self) -> SpatialRef {
        self.node(self.primary_camera_id.expect("primary camera not set"))
    }

    pub fn spatial(&self, handle: NodeHandle) -> &Spatial {
        self.nodes.get(handle).unwrap()
    }

    pub fn node(&self, handle: NodeHandle) -> SpatialRef {
        self.spatial(handle).node()
    }

    pub fn spatial_mut(&mut self, handle: NodeHandle) -> &mut Spatial {
        self.nodes.get_mut(handle).unwrap()
    }

    pub fn node_mut(&mut self, handle: NodeHandle) -> SpatialRefMut {
        self.spatial_mut(handle).node_mut()
    }
}

#[derive(Clone)]
pub struct Spatial {
    parent: Option<NodeHandle>,
    children: Vec<NodeHandle>,
    transform: Transform,
    world_transform: Transform,
    visible: bool,
    enabled: bool,
    node: Node,
    dirty: bool,
}

impl Spatial {
    pub fn new(node: impl Into<Node>) -> Self {
        Self {
            parent: None,
            children: Vec::new(),
            transform: Transform::default(),
            world_transform: Transform::default(),
            visible: true,
            enabled: true,
            node: node.into(),
            dirty: true,
        }
    }

    pub fn node(&self) -> SpatialRef {
        SpatialRef {
            parent: &self.parent,
            children: &self.children,
            transform: &self.transform,
            visible: &self.visible,
            enabled: &self.enabled,
            node: &self.node,
        }
    }

    pub fn node_mut(&mut self) -> SpatialRefMut {
        SpatialRefMut {
            parent: &mut self.parent,
            children: &mut self.children,
            transform: &mut self.transform,
            visible: &mut self.visible,
            enabled: &mut self.enabled,
            node: &mut self.node,
            dirty: &mut self.dirty,
        }
    }

    pub fn with_parent(mut self, parent: NodeHandle) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn with_children(mut self, children: Vec<NodeHandle>) -> Self {
        self.children = children;
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

    pub fn attach_child(&mut self, child: NodeHandle) {
        self.children.push(child);
    }

    pub fn detach_child(&mut self, child: NodeHandle) {
        let Some(position) = self.children.iter().position(|c| *c == child) else {
            return;
        };

        self.children.remove(position);
    }
}

pub struct SpatialRef<'a> {
    pub parent: &'a Option<NodeHandle>,
    pub children: &'a Vec<NodeHandle>,
    pub transform: &'a Transform,
    pub visible: &'a bool,
    pub enabled: &'a bool,
    pub node: &'a Node,
}

impl<'a> SpatialRef<'a> {
    fn transform(&self) -> &Transform {
        self.transform
    }
}

impl<'a> Deref for SpatialRef<'a> {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        self.node
    }
}

pub struct SpatialRefMut<'a> {
    pub parent: &'a mut Option<NodeHandle>,
    pub children: &'a mut Vec<NodeHandle>,
    pub transform: &'a mut Transform,
    pub visible: &'a mut bool,
    pub enabled: &'a mut bool,
    pub node: &'a mut Node,
    dirty: &'a mut bool,
}

impl<'a> SpatialRefMut<'a> {
    pub fn transform(&self) -> &Transform {
        self.transform
    }

    pub fn transform_mut(&mut self) -> &mut Transform {
        *self.dirty = true;
        self.transform
    }
}

impl<'a> Deref for SpatialRefMut<'a> {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        self.node
    }
}

impl<'a> DerefMut for SpatialRefMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.node
    }
}
