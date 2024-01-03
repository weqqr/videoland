use std::path::PathBuf;

use crate::shader::{Shader, ShaderCompiler, ShaderStage};

pub mod shader;

pub struct Vfs {
    shader_compiler: ShaderCompiler,
    root: PathBuf,
}

impl Vfs {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            shader_compiler: ShaderCompiler::new(),
            root: root.into(),
        }
    }

    fn real_path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    pub fn load_binary_sync(&self, path: &str) -> Vec<u8> {
        std::fs::read(self.real_path(path)).unwrap()
    }

    pub fn load_string_sync(&self, path: &str) -> String {
        std::fs::read_to_string(self.real_path(path)).unwrap()
    }

    pub fn load_shader_sync(&self, path: &str, stage: ShaderStage) -> Shader {
        let path = self.real_path(path);

        self.shader_compiler
            .compile_hlsl(path.to_str().unwrap(), stage)
            .unwrap()
    }
}
