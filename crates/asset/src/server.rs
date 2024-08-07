use crate::{
    handle::{ErasedHandle, Handle},
    Asset, AssetEvent, AssetId, AssetLoader, ErasedAssetLoader,
};
use crossbeam_channel::{Receiver, Sender};
use ecs::WinnyResource;
use parking_lot::RwLock;
use std::{
    any::TypeId,
    collections::HashMap,
    fmt::Debug,
    path::Path,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

/// Handle to the asset pipeline.
///
/// [`AssetServer`] is backed by an [`Arc`], so it may be sent cloned and sent between threads.
#[derive(WinnyResource, Debug, Default, Clone)]
pub struct AssetServer {
    loaders: Arc<RwLock<AssetLoaders>>,
}

impl AssetServer {
    pub(crate) fn register_loader<A: Asset>(
        &self,
        loader: impl AssetLoader,
        result: Sender<AssetEvent>,
        handler: Arc<AssetHandleCreator>,
    ) {
        self.loaders
            .write()
            .register_loader::<A>(loader, result, handler);
    }

    pub(crate) fn reload<P: AsRef<Path>>(&self, path: P) {
        self.loaders.write().reload(path);
    }

    pub fn load<A: Asset, P: AsRef<Path>>(&self, path: P) -> Handle<A> {
        self.loaders.write().load::<A, P>(path)
    }

    pub fn remove<A: Asset, P: AsRef<Path>>(&self, path: P) {
        self.loaders
            .write()
            .loaded_assets
            .remove(&path.as_ref().to_str().unwrap().to_owned());
    }
}

/// Type-erased storage for an [`AssetLoader`].
struct InternalAssetLoader {
    loader: Box<dyn ErasedAssetLoader>,
    handler: Arc<AssetHandleCreator>,
    result: Sender<AssetEvent>,
}

impl Debug for InternalAssetLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InternalAssetLoader")
            .field("handler", &self.handler)
            .field("result", &self.result)
            .finish_non_exhaustive()
    }
}

impl InternalAssetLoader {
    pub fn new(
        loader: impl AssetLoader,
        result: Sender<AssetEvent>,
        handler: Arc<AssetHandleCreator>,
    ) -> Self {
        Self {
            loader: Box::new(loader),
            handler,
            result,
        }
    }
}

/// Generates handles atomically for an [`Assets`] resource.
#[derive(Debug)]
pub struct AssetHandleCreator {
    next_id: AtomicU32,
    freed_rx: Receiver<u32>,
    freed_tx: Sender<u32>,
}

impl Default for AssetHandleCreator {
    fn default() -> Self {
        let (freed_tx, freed_rx) = crossbeam_channel::unbounded();

        Self {
            next_id: AtomicU32::new(0),
            freed_rx,
            freed_tx,
        }
    }
}

impl AssetHandleCreator {
    pub fn reserve(&self) -> ErasedHandle {
        let index = if let Ok(index) = self.freed_rx.try_recv() {
            index
        } else {
            self.next_id.fetch_add(1, Ordering::Relaxed)
        };

        ErasedHandle::new(AssetId(index))
    }

    pub fn remove(&self, id: AssetId) {
        if let Err(e) = self.freed_tx.send(id.0) {
            util::tracing::error!("Freed index tx error: {e}");
        }
    }
}

#[derive(Debug, Default)]
struct AssetLoaders {
    loaders: Vec<InternalAssetLoader>,
    type_to_loader: HashMap<TypeId, usize>,
    ext_to_loader: HashMap<&'static str, usize>,
    loaded_assets: HashMap<String, ErasedHandle>,
}

impl AssetLoaders {
    pub fn register_loader<A: Asset>(
        &mut self,
        loader: impl AssetLoader,
        result: Sender<AssetEvent>,
        handler: Arc<AssetHandleCreator>,
    ) {
        self.type_to_loader
            .insert(TypeId::of::<A>(), self.loaders.len());
        for ext in loader.extensions().iter() {
            self.ext_to_loader.insert(ext, self.loaders.len());
        }
        self.loaders
            .push(InternalAssetLoader::new(loader, result, handler));
    }

    pub fn load<A: Asset, P: AsRef<Path>>(&mut self, path: P) -> Handle<A> {
        if let Some(handle) = self.loaded_assets.get(path.as_ref().to_str().unwrap()) {
            return (*handle).into();
        }

        let file_ext = path
            .as_ref()
            .extension()
            .expect("file extension")
            .to_owned();
        let path = path.as_ref().to_str().unwrap().to_owned();

        let asset_type_id = TypeId::of::<A>();
        match self.type_to_loader.get(&asset_type_id) {
            Some(loader) => {
                let handle = self.loaders[*loader].loader.load(
                    &self.loaders[*loader].handler,
                    self.loaders[*loader].result.clone(),
                    path.clone(),
                    file_ext.to_str().unwrap().to_owned(),
                );
                self.loaded_assets.insert(path, handle);

                handle.into()
            }
            None => {
                util::tracing::error!(
                    "Could not find AssetLoader for file: {:?} of type: {:?}",
                    path,
                    std::any::type_name::<A>()
                );
                Handle::dangling()
            }
        }
    }

    pub fn reload<P: AsRef<Path>>(&mut self, path: P) {
        let file_ext = path
            .as_ref()
            .extension()
            .expect("file extension")
            .to_str()
            .unwrap();
        let path = path.as_ref().to_str().unwrap().to_owned();

        match self.ext_to_loader.get(file_ext) {
            Some(loader) => {
                let handle = self.loaded_assets.remove(&path).unwrap();
                self.loaders[*loader].handler.remove(handle.id());

                let new_handle = self.loaders[*loader].loader.load(
                    &self.loaders[*loader].handler,
                    self.loaders[*loader].result.clone(),
                    path.clone(),
                    file_ext.to_owned(),
                );
                assert_eq!(handle.id(), new_handle.id());
                self.loaded_assets.insert(path, new_handle);
            }
            None => {
                util::tracing::warn!("Could not find AssetLoader for file reload: {:?}", path);
            }
        }
    }
}
