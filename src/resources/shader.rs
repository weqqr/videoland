use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use hassle_rs::{Dxc, DxcCompiler, DxcIncludeHandler, DxcLibrary};

use crate::resources::FsDataLoader;

fn read_shader_source(loader: &FsDataLoader, path: &str) -> Result<String> {
    let data = loader.load_binary_from_raw_path(path)?;

    Ok(String::from_utf8(data)?)
}

struct IncludeHandler<'a> {
    loader: &'a FsDataLoader,
}

impl<'a> IncludeHandler<'a> {
    pub fn new(loader: &'a FsDataLoader) -> Self {
        Self { loader }
    }
}

impl<'a> DxcIncludeHandler for IncludeHandler<'a> {
    fn load_source(&mut self, path: String) -> Option<String> {
        read_shader_source(self.loader, &path).ok()
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

pub struct Shader {
    data: Vec<u8>,
}

impl Shader {
    pub fn data(&self) -> &[u32] {
        bytemuck::cast_slice(&self.data)
    }
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

    pub fn compile_hlsl(
        &self,
        loader: &FsDataLoader,
        path: &str,
        stage: ShaderStage,
    ) -> Result<Shader> {
        let source = read_shader_source(loader, path).context("reading shader source")?;

        let blob = self
            .library
            .create_blob_with_encoding_from_str(&source)
            .unwrap();

        let profile = stage.profile_name();
        let entry_point = stage.entry_point();
        let args = ["-HV 2021", "-I /", "-spirv"].as_slice();
        let mut include_handler = IncludeHandler::new(loader);
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
            Ok(v) => Ok(Shader {
                data: v.get_result().unwrap().to_vec(),
            }),
            Err(v) => {
                let message = self
                    .library
                    .get_blob_as_string(&v.0.get_error_buffer().unwrap().into())?;
                Err(anyhow!("shader error ({path}): {message}"))
            }
        }
    }
}
