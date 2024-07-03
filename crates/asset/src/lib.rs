use std::{
    collections::HashMap,
    fmt::Debug,
    fs::File,
    io::BufReader,
    marker::PhantomData,
    path::Path,
    sync::{Arc, Mutex},
};

use app::{app::App, plugins::Plugin};
use ecs::{
    new_dumb_drop, DumbVec, Events, ResMut, SparseArrayIndex, SparseSet, UnsafeWorldCell,
    WinnyComponent, WinnyEvent, WinnyResource,
};
use reader::ByteReader;

pub mod prelude;
pub mod reader;

// TODO: could become enum with strong and weak variants which determine
// dynamic loading behaviour
#[derive(WinnyComponent)]
pub struct Handle<A: Asset>(AssetId, PhantomData<A>);

impl<A: Asset> Debug for Handle<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Handle")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<A: Asset> Clone for Handle<A> {
    fn clone(&self) -> Self {
        Handle::new(self.id())
    }
}

impl<A: Asset> Handle<A> {
    pub fn new(id: AssetId) -> Self {
        Self(id, PhantomData)
    }

    pub fn id(&self) -> AssetId {
        self.0
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ErasedHandle(AssetId);

impl ErasedHandle {
    pub fn new(id: AssetId) -> Self {
        Self(id)
    }

    pub fn id(&self) -> AssetId {
        self.0
    }

    fn into_typed_handle<A: Asset>(self) -> Handle<A> {
        Handle::new(self.0)
    }
}

impl<A: Asset> Into<Handle<A>> for ErasedHandle {
    fn into(self) -> Handle<A> {
        self.into_typed_handle()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AssetId(u64);

impl AssetId {
    pub fn new(hash: u32, storage_index: u32) -> Self {
        Self(((hash as u64) << 32) | storage_index as u64)
    }

    // TODO: hash could be used to check if the asset has changed
    pub fn hash(&self) -> u32 {
        (self.0 >> 32) as u32
    }

    pub fn index(&self) -> u32 {
        self.0 as u32
    }
}

impl SparseArrayIndex for AssetId {
    fn to_index(&self) -> usize {
        self.index() as usize
    }
}

pub trait Asset: Send + Sync + 'static {}

#[derive(Debug)]
pub struct LoadedAsset<A: Asset> {
    pub asset: A,
    pub path: String,
    pub handle: ErasedHandle,
}

impl<A: Asset> LoadedAsset<A> {
    pub fn new(asset: A, path: String, handle: ErasedHandle) -> Self {
        Self {
            asset,
            path,
            handle,
        }
    }
}

use std::ops::{Deref, DerefMut};

impl<A: Asset> Deref for LoadedAsset<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.asset
    }
}

impl<A: Asset> DerefMut for LoadedAsset<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.asset
    }
}

impl<A: Asset> Into<ErasedLoadedAsset> for LoadedAsset<A> {
    fn into(self) -> ErasedLoadedAsset {
        ErasedLoadedAsset::new(self)
    }
}

#[derive(Debug)]
pub struct ErasedLoadedAsset {
    loaded_asset: DumbVec,
}

impl ErasedLoadedAsset {
    pub fn new<A: Asset>(asset: LoadedAsset<A>) -> Self {
        let mut loaded_asset = DumbVec::new(
            std::alloc::Layout::new::<LoadedAsset<A>>(),
            1,
            new_dumb_drop::<LoadedAsset<A>>(),
        );
        let _ = loaded_asset.push(asset);

        Self { loaded_asset }
    }
}

// An asset will only ever be read from.
unsafe impl Sync for ErasedLoadedAsset {}
unsafe impl Send for ErasedLoadedAsset {}

impl<A: Asset> Into<LoadedAsset<A>> for ErasedLoadedAsset {
    fn into(mut self) -> LoadedAsset<A> {
        self.loaded_asset.pop_unchecked()
    }
}

#[derive(WinnyResource)]
pub struct Assets<A>
where
    A: Asset,
{
    storage: SparseSet<AssetId, LoadedAsset<A>>,
}

impl<A> Assets<A>
where
    A: Asset,
{
    pub fn new() -> Self {
        Self {
            storage: SparseSet::new(),
        }
    }

    pub fn insert(&mut self, asset: LoadedAsset<A>, id: AssetId) {
        self.storage.insert(id, asset);
    }

    pub fn get(&self, handle: &Handle<A>) -> Option<&LoadedAsset<A>> {
        self.storage.get(&handle.id())
    }
}

struct InternalAssetLoader {
    loader: Box<dyn ErasedAssetLoader>,
    async_dispatch: Arc<Mutex<AsyncAssetSender>>,
    handler: AssetHandleCreator,
}

impl InternalAssetLoader {
    pub fn new(loader: impl ErasedAssetLoader, dispatch: AsyncAssetSender) -> Self {
        Self {
            handler: AssetHandleCreator::new(),
            async_dispatch: Arc::new(Mutex::new(dispatch)),
            loader: Box::new(loader),
        }
    }
}

pub struct AssetHandleCreator {
    next_id: u32,
    freed_indexes: Vec<u32>,
}

impl AssetHandleCreator {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            freed_indexes: Vec::new(),
        }
    }

    pub fn new_id(&mut self) -> ErasedHandle {
        let index = if let Some(index) = self.freed_indexes.iter().next() {
            *index
        } else {
            let index = self.next_id;
            self.next_id += 1;
            index
        };

        // TODO: hash this shit
        ErasedHandle::new(AssetId::new(0, index))
    }
}

struct AssetLoaders {
    loaders: Vec<InternalAssetLoader>,
    ext_to_loader: Vec<(Vec<String>, usize)>,
    loaded_assets: HashMap<String, ErasedHandle>,
}

impl AssetLoaders {
    pub fn new() -> Self {
        Self {
            loaders: Vec::new(),
            ext_to_loader: Vec::new(),
            loaded_assets: HashMap::new(),
        }
    }

    pub fn register_loader(
        &mut self,
        loader: impl ErasedAssetLoader,
        result: Box<dyn FnMut(Result<ErasedLoadedAsset, ()>)>,
    ) {
        self.ext_to_loader
            .push((loader.extensions(), self.loaders.len()));
        self.loaders.push(InternalAssetLoader::new(
            loader,
            AsyncAssetSender::new(Box::new(result)),
        ));
    }

    pub fn load<A: Asset, P: AsRef<Path>>(&mut self, asset_folder: String, path: P) -> Handle<A> {
        if let Some(handle) = self.loaded_assets.get(path.as_ref().to_str().unwrap()) {
            return handle.into_typed_handle();
        }

        let file_ext = path
            .as_ref()
            .extension()
            .expect("file extension")
            .to_owned();
        let path = path.as_ref().to_str().unwrap().to_owned();

        let loader_index = self
            .ext_to_loader
            .iter()
            .find(|(ext, _)| ext.contains(&file_ext.to_str().unwrap().to_string()))
            .map(|(_, i)| i)
            .expect("valid file");
        let f =
            std::fs::File::open(Path::new(&asset_folder).join(path.clone())).expect("valid file");
        let reader = ByteReader::new(BufReader::new(f));
        let internal_loader = &mut self.loaders[*loader_index];

        let handle = internal_loader.loader.load(
            reader,
            internal_loader.async_dispatch.clone(),
            &mut internal_loader.handler,
            path.clone(),
            file_ext.to_str().unwrap().to_owned(),
        );
        self.loaded_assets.insert(path, handle.clone());

        handle.into()
    }
}

#[derive(WinnyResource)]
pub struct AssetServer {
    asset_folder: String,
    loaders: AssetLoaders,
}

impl AssetServer {
    pub fn new(asset_folder: String) -> Self {
        Self {
            asset_folder,
            loaders: AssetLoaders::new(),
        }
    }

    pub fn register_loader(
        &mut self,
        loader: impl ErasedAssetLoader,
        result: Box<dyn FnMut(Result<ErasedLoadedAsset, ()>)>,
    ) {
        self.loaders.register_loader(loader, result);
    }

    pub fn load<A: Asset, P: AsRef<Path>>(&mut self, path: P) -> Handle<A> {
        self.loaders.load(self.asset_folder.clone(), path)
    }
}

pub trait AssetApp {
    fn register_asset_loader<A: Asset>(&mut self, loader: impl AssetLoader) -> &mut Self;
}

impl AssetApp for App {
    fn register_asset_loader<A: Asset>(&mut self, loader: impl AssetLoader) -> &mut Self {
        let assets: Assets<A> = Assets::new();
        self.insert_resource(assets);
        self.world_mut().insert_resource(AssetEvents::<A>::new());

        let (asset_result_tx, asset_result_rx) = std::sync::mpsc::channel();
        let arrx = ecs::threads::ChannelReciever::new(asset_result_rx);

        self.add_systems(
            ecs::Schedule::PreUpdate,
            move |asset_events: ResMut<AssetEvents<A>>, mut assets: ResMut<Assets<A>>| {
                if let Ok(()) = arrx.try_recv() {
                    for event in asset_events.events.lock().unwrap().read() {
                        match event {
                            AssetEvent::Err => {
                                // TODO: Handle asset errors
                                logger::error!(
                                    "Failed to load asset [{}] => {:?}",
                                    std::any::type_name::<A>(),
                                    "unknown"
                                );
                            }
                            AssetEvent::Loaded { asset } => {
                                logger::info!(
                                    "Loaded asset [{}] => {:?}",
                                    std::any::type_name::<A>(),
                                    asset.path
                                );
                                let id = asset.handle.id();
                                assets.insert(asset, id);
                            }
                        }
                    }
                }
            },
        );

        let mut asset_event_writer =
            AssetEventWriter::<A>::new(unsafe { self.world().as_unsafe_world() });

        let result = move |result: Result<ErasedLoadedAsset, ()>| {
            asset_event_writer.send(if let Ok(asset) = result {
                let asset = asset.into();
                AssetEvent::Loaded { asset }
            } else {
                AssetEvent::Err
            });
            // TODO: will this fail sometimes?
            let _ = asset_result_tx.send(());
        };

        let mut server = self.world_mut().resource_mut::<AssetServer>();
        server.register_loader(loader, Box::new(result));

        self
    }
}

#[derive(Debug, WinnyEvent)]
pub enum AssetEvent<A: Asset> {
    Loaded { asset: LoadedAsset<A> },
    Err,
}

#[derive(Debug, WinnyResource)]
pub struct AssetEvents<A: Asset> {
    pub events: Mutex<Events<AssetEvent<A>>>,
}

impl<A: Asset> AssetEvents<A> {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Events::new()),
        }
    }
}

pub struct AssetEventWriter<'w, A: Asset> {
    // This is safe if and only if there is only one world with one asset loader
    // for any given asset.
    //
    // This event writer will be captured in a closure and passed to the
    // asset loader for loading.
    events: ResMut<'w, AssetEvents<A>>,
}

impl<'w, A: Asset> AssetEventWriter<'w, A> {
    pub fn new(world: UnsafeWorldCell<'w>) -> Self {
        let id = unsafe { world.read_and_write() }.get_resource_id::<AssetEvents<A>>();
        Self {
            events: ResMut::new(world, id),
        }
    }

    pub fn send(&mut self, event: AssetEvent<A>) {
        let _ = self.events.events.lock().unwrap().push(event);
    }
}

pub trait ErasedAssetEventWriter {}

impl<A: Asset> ErasedAssetEventWriter for AssetEventWriter<'_, A> {}

pub trait AssetLoader: Send + Sync + 'static {
    type Asset: Asset;

    fn load(reader: ByteReader<File>, ext: &str) -> Result<Self::Asset, ()>;
    fn extensions(&self) -> Vec<String>;
}

pub trait ErasedAssetLoader: Send + Sync + 'static {
    fn load(
        &self,
        reader: ByteReader<File>,
        sender: Arc<Mutex<AsyncAssetSender>>,
        handler: &mut AssetHandleCreator,
        path: String,
        ext: String,
    ) -> ErasedHandle;
    fn extensions(&self) -> Vec<String>;
}

pub struct AsyncAssetSender {
    result: Box<dyn FnMut(Result<ErasedLoadedAsset, ()>)>,
}

impl AsyncAssetSender {
    pub fn new(result: Box<dyn FnMut(Result<ErasedLoadedAsset, ()>)>) -> Self {
        Self { result }
    }

    pub fn send_result<A: Asset>(&mut self, result: Result<LoadedAsset<A>, ()>) {
        (self.result)(result.map(|a| a.into()))
    }
}

unsafe impl Sync for AsyncAssetSender {}
unsafe impl Send for AsyncAssetSender {}

impl<L: AssetLoader> ErasedAssetLoader for L {
    fn load(
        &self,
        reader: ByteReader<File>,
        sender: Arc<Mutex<AsyncAssetSender>>,
        handler: &mut AssetHandleCreator,
        path: String,
        ext: String,
    ) -> ErasedHandle {
        let handle = handler.new_id();

        std::thread::spawn(move || {
            let asset = if let Ok(asset) = L::load(reader, ext.as_str()) {
                Ok(LoadedAsset::new(asset, path, handle.clone()))
            } else {
                Err(())
            };
            sender.lock().unwrap().send_result(asset);
        });

        handle
    }

    fn extensions(&self) -> Vec<String> {
        self.extensions()
    }
}

#[derive(Debug, Clone)]
pub struct AssetLoaderPlugin {
    pub asset_folder: String,
}

impl Plugin for AssetLoaderPlugin {
    fn build(&mut self, app: &mut App) {
        let server = AssetServer::new(self.asset_folder.clone());
        app.insert_resource(server);
    }
}
