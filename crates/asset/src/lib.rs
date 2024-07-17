use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    fs::File,
    io::BufReader,
    marker::PhantomData,
    path::Path,
    sync::{mpsc::TryRecvError, Arc, Mutex},
};

use app::{app::App, plugins::Plugin};
use ecs::{
    DumbVec, EventWriter, ResMut, SparseArrayIndex, SparseSet, WinnyComponent, WinnyEvent,
    WinnyResource,
};
use reader::ByteReader;

use util::tracing::{error, info, trace, trace_span};

pub mod prelude;
pub mod reader;

// TODO: could become enum with strong and weak variants which determine
// dynamic loading behaviour
#[derive(WinnyComponent, Copy)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    fn index(&self) -> usize {
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
        let mut loaded_asset = DumbVec::with_capacity::<LoadedAsset<A>>(1);
        unsafe { loaded_asset.push::<LoadedAsset<A>>(asset) };

        Self { loaded_asset }
    }
}

// An asset will only ever be read from.
unsafe impl Sync for ErasedLoadedAsset {}
unsafe impl Send for ErasedLoadedAsset {}

impl<A: Asset> Into<LoadedAsset<A>> for ErasedLoadedAsset {
    fn into(mut self) -> LoadedAsset<A> {
        unsafe { self.loaded_asset.pop::<LoadedAsset<A>>() }
    }
}

#[derive(WinnyResource)]
pub struct Assets<A>
where
    A: Asset,
{
    storage: SparseSet<AssetId, LoadedAsset<A>>,
}

impl<A: Asset> Debug for Assets<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Assets").finish()
    }
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

impl Debug for InternalAssetLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ItnernalAssetLoader").finish()
    }
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

#[derive(Debug)]
struct AssetLoaders {
    loaders: Vec<InternalAssetLoader>,
    ext_to_loader: Vec<(Vec<&'static str>, usize)>,
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
        result: Box<dyn FnMut(ErasedHandle, Result<ErasedLoadedAsset, (String, AssetLoaderError)>)>,
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
            trace!("found loaded asset, returning");
            return handle.into_typed_handle();
        }

        trace!("loaded asset not found");
        let file_ext = path
            .as_ref()
            .extension()
            .expect("file extension")
            .to_owned();
        let path = path.as_ref().to_str().unwrap().to_owned();

        let loader_index = self
            .ext_to_loader
            .iter()
            .find(|(ext, _)| ext.contains(&file_ext.to_str().unwrap()))
            .map(|(_, i)| i)
            .expect("valid ext");
        let f = std::fs::File::open(Path::new(&asset_folder).join(path.clone()))
            .expect(format!("valid file: {}", path).as_str());
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

#[derive(Debug, WinnyResource)]
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
        result: Box<dyn FnMut(ErasedHandle, Result<ErasedLoadedAsset, (String, AssetLoaderError)>)>,
    ) {
        self.loaders.register_loader(loader, result);
    }

    pub fn load<A: Asset, P: AsRef<Path>>(&mut self, path: P) -> Handle<A> {
        let _span = trace_span!("asset load").entered();
        self.loaders.load(self.asset_folder.clone(), path)
    }
}

pub trait AssetApp {
    fn register_asset_loader<A: Asset>(&mut self, loader: impl AssetLoader) -> &mut Self;
}

impl AssetApp for App {
    fn register_asset_loader<A: Asset>(&mut self, loader: impl AssetLoader) -> &mut Self {
        let assets: Assets<A> = Assets::new();
        self.insert_resource(assets)
            .register_event::<AssetLoaderEvent<A>>();

        let (asset_result_tx, asset_result_rx) = std::sync::mpsc::channel();
        let asset_result_rx = ecs::threads::ChannelReciever::new(asset_result_rx);

        self.add_systems(
            ecs::Schedule::Platform,
            move |mut assets: ResMut<Assets<A>>,
                  mut asset_loader_events: EventWriter<AssetLoaderEvent<A>>| {
                match asset_result_rx.try_recv() {
                    Ok(event) => match event {
                        AssetEvent::Err { handle } => {
                            asset_loader_events.send(AssetLoaderEvent::Err { handle })
                        }
                        AssetEvent::Loaded { asset } => {
                            info!(
                                "Loaded asset [{}]: {:?}",
                                std::any::type_name::<A>(),
                                asset.path
                            );

                            asset_loader_events.send(AssetLoaderEvent::Loaded {
                                handle: asset.handle.into(),
                            });

                            let id = asset.handle.id();
                            assets.insert(asset, id);
                        }
                    },
                    Err(e) => match e {
                        TryRecvError::Empty => (),
                        TryRecvError::Disconnected => {
                            error!("Asset sender channel closed");
                            panic!();
                        }
                    },
                }
            },
        );

        let result =
            move |handle: ErasedHandle,
                  result: Result<ErasedLoadedAsset, (String, AssetLoaderError)>| {
                if let Err(e) = asset_result_tx.send(match result {
                    Ok(asset) => {
                        let asset = asset.into();
                        AssetEvent::Loaded { asset }
                    }
                    Err((path, e)) => {
                        error!("{}: {:?}", e, path);

                        AssetEvent::Err {
                            handle: handle.into(),
                        }
                    }
                }) {
                    error!("Asset reciever channel closed: {}", e);
                    panic!();
                }
            };

        let mut server = self.world_mut().resource_mut::<AssetServer>();
        server.register_loader(loader, Box::new(result));
        drop(server);

        self
    }
}

#[derive(WinnyEvent)]
pub enum AssetLoaderEvent<A: Asset> {
    Loaded { handle: Handle<A> },
    Err { handle: Handle<A> },
}

impl<A: Asset> Debug for AssetLoaderEvent<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Loaded { handle } => write!(f, "AssetLoaderEvent::Loaded: {:?}", handle),
            Self::Err { handle } => write!(f, "AssetLoaderEvent::Err: {:?}", handle),
        }
    }
}

enum AssetEvent<A: Asset + Send + Sync> {
    Loaded { asset: LoadedAsset<A> },
    Err { handle: Handle<A> },
}

impl<A: Asset> Debug for AssetEvent<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetEvent").finish()
    }
}

#[derive(Debug)]
pub enum AssetLoaderError {
    UnsupportedFileExtension,
    FileNotFound,
    FailedToParse,
}

impl Display for AssetLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedFileExtension => {
                write!(f, "File extension is not supported")
            }
            Self::FileNotFound => {
                write!(f, "File could not be found")
            }
            Self::FailedToParse => {
                write!(f, "File parser failed")
            }
        }
    }
}

impl std::error::Error for AssetLoaderError {}

pub trait AssetLoader: Send + Sync + 'static {
    type Asset: Asset;

    fn load(
        reader: ByteReader<File>,
        path: String,
        ext: &str,
    ) -> Result<Self::Asset, AssetLoaderError>;
    fn extensions(&self) -> Vec<&'static str>;
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
    fn extensions(&self) -> Vec<&'static str>;
}

pub struct AsyncAssetSender {
    result: Box<dyn FnMut(ErasedHandle, Result<ErasedLoadedAsset, (String, AssetLoaderError)>)>,
}

impl AsyncAssetSender {
    pub fn new(
        result: Box<dyn FnMut(ErasedHandle, Result<ErasedLoadedAsset, (String, AssetLoaderError)>)>,
    ) -> Self {
        Self { result }
    }

    pub fn send_result<A: Asset>(
        &mut self,
        handle: ErasedHandle,
        result: Result<LoadedAsset<A>, (String, AssetLoaderError)>,
    ) {
        (self.result)(handle, result.map(|a| a.into()))
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
            let _span = trace_span!("load thread").entered();
            let result = L::load(reader, path.clone(), ext.as_str())
                .map(|asset| LoadedAsset::new(asset, path.clone(), handle.clone()));
            sender
                .lock()
                .unwrap()
                .send_result(handle, result.map_err(|e| (path, e)));
        });

        handle
    }

    fn extensions(&self) -> Vec<&'static str> {
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
