use std::sync::Arc;

use rayon::ThreadPool;
use videoland_ap::model::Model;
use videoland_ap::{AssetId, Vfs};

use crossbeam_channel as channel;
use videoland_ecs::ResMut;

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
