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
    pub fn from_data(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}
