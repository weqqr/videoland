#[derive(Debug, Clone, Copy)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

pub struct Shader {
    data: Vec<u8>,
}

impl Shader {
    pub fn from_spirv_unchecked(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn spirv(&self) -> &[u32] {
        bytemuck::cast_slice(&self.data)
    }
}
