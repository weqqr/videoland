use glam::Mat4;

pub trait Camera {
    fn world_transform(&self) -> Mat4;
}
