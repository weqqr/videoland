use std::path::PathBuf;
use std::sync::Arc;

use crate::asset::model::Model;
use crate::asset::shader::{Shader, ShaderStage};
use crate::asset::{AssetId, Vfs};
use crate::core::ResMut;
use hassle_rs::{Dxc, DxcCompiler, DxcIncludeHandler, DxcLibrary, HassleError};
use rayon::ThreadPool;

use crossbeam_channel as channel;

pub struct Loader {
    vfs: Arc<Vfs>,
    thread_pool: Arc<ThreadPool>,

    model_tx: channel::Sender<LoadResponse<Model>>,
    model_rx: channel::Receiver<LoadResponse<Model>>,
}

enum LoadResponse<T> {
    Done((AssetId, T)),
    Error(Box<dyn std::error::Error + Send>),
}

impl Loader {
    pub fn new(vfs: Arc<Vfs>, thread_pool: Arc<ThreadPool>) -> Self {
        let (model_tx, model_rx) = channel::unbounded();

        Self {
            vfs,
            thread_pool,

            model_tx,
            model_rx,
        }
    }

    pub fn vfs(&self) -> &Vfs {
        &self.vfs
    }

    pub fn load_model_async(&self, path: &str) -> AssetId {
        let id = self.vfs.acquire_asset_id_for_path(path);

        let path = path.to_owned();

        let model_tx = self.model_tx.clone();

        self.thread_pool.spawn(move || {
            let response = std::fs::read(path)
                .map(|data| LoadResponse::Done((id, Model::from_obj(&data))))
                .unwrap_or_else(|err| LoadResponse::Error(Box::new(err)));

            model_tx.send(response).unwrap();
        });

        id
    }
}

pub fn poll(loader: ResMut<Loader>) {
    for load_response in loader.model_rx.try_iter() {
        match load_response {
            LoadResponse::Done((id, model)) => {
                println!("loaded: {:?}", id);
            }
            LoadResponse::Error(err) => {
                println!("error: {}", err);
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("HLSL error:\n{0}")]
    Compile(String),

    #[error("shader compiler error: {0}")]
    Hassle(#[from] HassleError),

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

fn shader_profile_name(stage: ShaderStage) -> &'static str {
    match stage {
        ShaderStage::Vertex => "vs_6_0",
        ShaderStage::Fragment => "ps_6_0",
        ShaderStage::Compute => "cs_6_0",
    }
}

fn shader_entry_point(stage: ShaderStage) -> &'static str {
    match stage {
        ShaderStage::Vertex => "vs_main",
        ShaderStage::Fragment => "fs_main",
        ShaderStage::Compute => "cs_main",
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

        let profile = shader_profile_name(stage);
        let entry_point = shader_entry_point(stage);
        let args = ["-HV 2021", "-I /"].as_slice();
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

                Ok(Shader::from_data(data))
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
