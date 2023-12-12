use std::path::PathBuf;

use hassle_rs::{Dxc, DxcCompiler, DxcIncludeHandler, DxcLibrary};

use crate::render2::Shader;

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("HLSL error:\n{0}")]
    Compile(String),

    #[error("shader compiler error: {0}")]
    Hassle(#[from] hassle_rs::HassleError),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

fn read_shader_source(path: &str) -> Result<String, Error> {
    Ok(std::fs::read_to_string(path)?)
}

struct IncludeHandler {}

impl IncludeHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl DxcIncludeHandler for IncludeHandler {
    fn load_source(&mut self, path: String) -> Option<String> {
        read_shader_source(&path).ok()
    }
}

#[allow(dead_code)]
pub struct ShaderCompiler {
    library: DxcLibrary,
    compiler: DxcCompiler,
    dxc: Dxc,
}

pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

impl ShaderStage {
    pub fn profile_name(&self) -> &'static str {
        match self {
            ShaderStage::Vertex => "vs_6_0",
            ShaderStage::Fragment => "ps_6_0",
            ShaderStage::Compute => "cs_6_0",
        }
    }

    pub fn entry_point(&self) -> &'static str {
        match self {
            ShaderStage::Vertex => "vs_main",
            ShaderStage::Fragment => "fs_main",
            ShaderStage::Compute => "cs_main",
        }
    }
}

impl ShaderCompiler {
    pub fn new() -> Self {
        let dxc = Dxc::new(Some(PathBuf::from("bin"))).unwrap();
        let compiler = dxc.create_compiler().unwrap();
        let library = dxc.create_library().unwrap();

        Self {
            dxc,
            compiler,
            library,
        }
    }

    pub fn compile_hlsl(&self, path: &str, stage: ShaderStage) -> Result<Shader, Error> {
        let source = read_shader_source(path)?;

        let blob = self
            .library
            .create_blob_with_encoding_from_str(&source)
            .unwrap();

        let profile = stage.profile_name();
        let entry_point = stage.entry_point();
        let args = ["-HV 2021", "-I /", "-spirv"].as_slice();
        let mut include_handler = IncludeHandler::new();
        let defines = &[];
        let result = self.compiler.compile(
            &blob,
            path,
            entry_point,
            profile,
            args,
            Some(&mut include_handler),
            defines,
        );

        match result {
            Ok(v) => {
                let data = v.get_result().unwrap().to_vec();

                Ok(Shader::from_spirv_unchecked(data))
            }
            Err(err) => {
                let message = self
                    .library
                    .get_blob_as_string(&err.0.get_error_buffer().unwrap().into())?;

                Err(Error::Compile(message))
            }
        }
    }
}
