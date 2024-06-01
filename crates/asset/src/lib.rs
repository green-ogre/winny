use std::{
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
    WinnyEvent, WinnyResource,
};
use reader::ByteReader;

pub mod prelude;
pub mod reader;

// TODO: could become enum with strong and weak variants which determine
// dynamic loading behaviour
#[derive(Debug, Clone)]
pub struct Handle<A: Asset>(AssetId, PhantomData<A>);

impl<A: Asset> Handle<A> {
    pub fn new(id: AssetId) -> Self {
        Self(id, PhantomData)
    }

    pub fn id(&self) -> AssetId {
        self.0
    }
}

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
    asset: A,
}

impl<A: Asset> LoadedAsset<A> {
    pub fn new(asset: A) -> Self {
        Self { asset }
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

    pub fn get(&self, handle: Handle<A>) -> &LoadedAsset<A> {
        self.storage.get(&handle.id()).unwrap()
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
}

impl AssetLoaders {
    pub fn new() -> Self {
        Self {
            loaders: Vec::new(),
            ext_to_loader: Vec::new(),
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
        let file_ext = path.as_ref().extension().expect("file extension");
        let loader_index = self
            .ext_to_loader
            .iter()
            .find(|(ext, _)| ext.contains(&file_ext.to_str().unwrap().to_string()))
            .map(|(_, i)| i)
            .expect("valid file");
        let f = std::fs::File::open(Path::new(&asset_folder).join(path)).expect("valid file");
        let reader = ByteReader::new(BufReader::new(f));
        let internal_loader = &mut self.loaders[*loader_index];
        internal_loader
            .loader
            .load(
                reader,
                internal_loader.async_dispatch.clone(),
                &mut internal_loader.handler,
            )
            .into()
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
        let mut asset_event_writer =
            AssetEventWriter::<A>::new(unsafe { self.world().as_unsafe_world() });

        let result = move |result: Result<ErasedLoadedAsset, ()>| {
            asset_event_writer.send(if let Ok(asset) = result {
                let asset = asset.into();
                AssetEvent::Loaded { asset }
            } else {
                AssetEvent::Err
            });
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
    events: Mutex<Events<AssetEvent<A>>>,
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
        let id = unsafe { world.read_only() }.get_resource_id::<AssetEvents<A>>();
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

    fn load(reader: ByteReader<File>) -> Result<LoadedAsset<Self::Asset>, ()>;
    fn extensions(&self) -> Vec<String>;
}

pub trait ErasedAssetLoader: Send + Sync + 'static {
    fn load(
        &self,
        reader: ByteReader<File>,
        sender: Arc<Mutex<AsyncAssetSender>>,
        handler: &mut AssetHandleCreator,
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
    ) -> ErasedHandle {
        std::thread::spawn(move || {
            let asset = L::load(reader);
            sender.lock().unwrap().send_result(asset);
        });

        handler.new_id()
    }
    fn extensions(&self) -> Vec<String> {
        self.extensions()
    }
}

pub struct AssetLoaderPlugin {
    pub asset_folder: String,
}

impl Plugin for AssetLoaderPlugin {
    fn build(&mut self, app: &mut App) {
        let server = AssetServer::new(self.asset_folder.clone());
        app.insert_resource(server);
    }
}
