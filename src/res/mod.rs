use std::path::PathBuf;

pub struct Resources {
    root: PathBuf,
}

impl Resources {
    pub fn read(&self, path: &str) -> Vec<u8> {
        std::fs::read(self.root.join(path)).unwrap()
    }
}

pub struct Vertex {
    pub a: f32,
}

pub struct Mesh {
    pub vertex_data: Vec<f32>,
}
