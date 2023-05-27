use ash::vk;

#[derive(Clone, Copy)]
pub struct ImageView {
    pub(super) image_view: vk::ImageView,
    pub(super) width: u32,
    pub(super) height: u32,
}

impl ImageView {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}
