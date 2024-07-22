use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    future::Future,
    io::{BufReader, Cursor},
    marker::PhantomData,
    path::Path,
    sync::{
        atomic::{AtomicU32, Ordering},
        mpsc::TryRecvError,
        Arc,
    },
};

use app::{app::App, plugins::Plugin};
use ecs::{
    DumbVec, EventWriter, Res, ResMut, SparseArrayIndex, SparseSet, WinnyComponent, WinnyEvent,
    WinnyResource,
};
use reader::ByteReader;

use render::{RenderConfig, RenderContext, RenderDevice, RenderQueue};
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

impl<A: Asset> PartialEq for Handle<A> {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
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
    async_dispatch: AssetFuture,
    handler: AssetHandleCreator,
}

impl Debug for InternalAssetLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ItnernalAssetLoader").finish()
    }
}

impl InternalAssetLoader {
    pub fn new(loader: impl ErasedAssetLoader, async_dispatch: AssetFuture) -> Self {
        Self {
            handler: AssetHandleCreator::new(),
            async_dispatch,
            loader: Box::new(loader),
        }
    }
}

#[derive(Clone)]
pub struct AssetFuture {
    future: Arc<AsyncAssetSender>,
}

impl AssetFuture {
    pub fn new(
        future: impl Fn(ErasedHandle, Result<ErasedLoadedAsset, (String, AssetLoaderError)>) + 'static,
    ) -> Self {
        Self {
            future: Arc::new(AsyncAssetSender::new(future)),
        }
    }

    pub fn send_result<A: Asset>(
        &self,
        handle: ErasedHandle,
        result: Result<LoadedAsset<A>, (String, AssetLoaderError)>,
    ) {
        self.future.send_result(handle, result);
    }
}

pub struct AssetHandleCreator {
    next_id: AtomicU32,
    freed_indexes: Vec<u32>,
}

impl AssetHandleCreator {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU32::new(0),
            freed_indexes: Vec::new(),
        }
    }

    pub fn new_id(&self) -> ErasedHandle {
        let index = if let Some(index) = self.freed_indexes.iter().next() {
            *index
        } else {
            let index = self.next_id.fetch_add(1, Ordering::AcqRel);
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

    pub fn register_loader(&mut self, loader: impl ErasedAssetLoader, future: AssetFuture) {
        self.ext_to_loader
            .push((loader.extensions(), self.loaders.len()));
        self.loaders.push(InternalAssetLoader::new(loader, future));
    }

    pub fn load<A: Asset, P: AsRef<Path>>(
        &self,
        _asset_folder: String,
        path: P,
        context: RenderContext,
    ) -> Handle<A> {
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
            .expect("unsupported file type");
        // TODO: handle error
        let internal_loader = &self.loaders[*loader_index];
        let handle = internal_loader.loader.load(
            context,
            internal_loader.async_dispatch.clone(),
            &internal_loader.handler,
            path.clone(),
            file_ext.to_str().unwrap().to_owned(),
        );
        // TODO: add loaded assets

        handle.into()
    }
}

#[derive(Debug, WinnyResource)]
pub struct AssetServer {
    asset_folder: String,
    loaders: AssetLoaders,
    render_context: Option<RenderContext>,
}

impl AssetServer {
    pub fn new(asset_folder: String) -> Self {
        Self {
            asset_folder,
            loaders: AssetLoaders::new(),
            render_context: None,
        }
    }

    pub fn register_loader(&mut self, loader: impl ErasedAssetLoader, future: AssetFuture) {
        self.loaders.register_loader(loader, future);
    }

    pub fn load<A: Asset, P: AsRef<Path>>(&self, path: P) -> Handle<A> {
        let _span = trace_span!("asset load").entered();
        self.loaders.load(
            self.asset_folder.clone(),
            path,
            self.render_context
                .as_ref()
                .expect("Do not load assets before the `StartUp` schedule")
                .clone(),
        )
    }

    fn insert_context(&mut self, context: RenderContext) {
        self.render_context = Some(context);
    }
}

fn update_asset_server_context(
    mut server: ResMut<AssetServer>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    config: Res<RenderConfig>,
) {
    // cheap to make
    server.insert_context(RenderContext {
        queue: queue.clone(),
        device: device.clone(),
        config: config.clone(),
    })
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
        server.register_loader(loader, AssetFuture::new(result));
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
    FailedToBuild,
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
            Self::FailedToBuild => {
                write!(f, "Asset builder failed")
            }
        }
    }
}

impl std::error::Error for AssetLoaderError {}

pub trait AssetLoader: Send + Sync + 'static {
    type Asset: Asset;

    fn load(
        context: RenderContext,
        reader: ByteReader<Cursor<Vec<u8>>>,
        path: String,
        ext: &str,
    ) -> impl Future<Output = Result<Self::Asset, AssetLoaderError>>;
    fn extensions(&self) -> Vec<&'static str>;
}

pub trait ErasedAssetLoader: Send + Sync + 'static {
    fn load(
        &self,
        context: RenderContext,
        sender: AssetFuture,
        handler: &AssetHandleCreator,
        path: String,
        ext: String,
    ) -> ErasedHandle;
    fn extensions(&self) -> Vec<&'static str>;
}

pub struct AsyncAssetSender {
    result: Box<dyn Fn(ErasedHandle, Result<ErasedLoadedAsset, (String, AssetLoaderError)>)>,
}

impl AsyncAssetSender {
    pub fn new(
        result: impl Fn(ErasedHandle, Result<ErasedLoadedAsset, (String, AssetLoaderError)>) + 'static,
    ) -> Self {
        Self {
            result: Box::new(result),
        }
    }

    pub fn send_result<A: Asset>(
        &self,
        handle: ErasedHandle,
        result: Result<LoadedAsset<A>, (String, AssetLoaderError)>,
    ) {
        (self.result)(handle, result.map(|a| a.into()))
    }
}

unsafe impl Sync for AsyncAssetSender {}
unsafe impl Send for AsyncAssetSender {}

#[cfg(not(target_arch = "wasm32"))]
impl<L: AssetLoader> ErasedAssetLoader for L {
    fn load(
        &self,
        context: RenderContext,
        sender: AssetFuture,
        handler: &AssetHandleCreator,
        path: String,
        ext: String,
    ) -> ErasedHandle {
        let handle = handler.new_id();

        std::thread::spawn(move || {
            let _span = trace_span!("load thread").entered();
            let binary = pollster::block_on(load_binary(path.as_str())).unwrap();
            let reader = ByteReader::new(BufReader::new(Cursor::new(binary)));

            let result = pollster::block_on(L::load(context, reader, path.clone(), ext.as_str()))
                .map(|asset| LoadedAsset::new(asset, path.clone(), handle.clone()));
            sender.send_result(handle, result.map_err(|e| (path, e)));
        });

        handle
    }

    fn extensions(&self) -> Vec<&'static str> {
        self.extensions()
    }
}

#[cfg(target_arch = "wasm32")]
impl<L: AssetLoader> ErasedAssetLoader for L {
    fn load(
        &self,
        context: RenderContext,
        sender: AssetFuture,
        handler: &AssetHandleCreator,
        path: String,
        ext: String,
    ) -> ErasedHandle {
        let handle = handler.new_id();

        wasm_bindgen_futures::spawn_local(async move {
            info!("reading file: {:?}", path);
            let binary = load_binary(path.as_str()).await.unwrap();
            info!("finished reading file: {:?}", path);
            let reader = ByteReader::new(BufReader::new(Cursor::new(binary)));
            let result = L::load(context, reader, path.clone(), ext.as_str())
                .await
                .map(|asset| LoadedAsset::new(asset, path.clone(), handle.clone()));
            sender.send_result(handle, result.map_err(|e| (path, e)));
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
        app.insert_resource(server)
            .add_systems(ecs::Schedule::PreStartUp, update_asset_server_context)
            .add_systems(ecs::Schedule::Platform, update_asset_server_context);
    }
}

#[cfg(target_arch = "wasm32")]
fn format_url(file_name: &str) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let mut origin = location.origin().unwrap();
    if !origin.ends_with("res") {
        origin = format!("{}/res", origin);
    }
    let base = reqwest::Url::parse(&format!("{}/", origin,)).unwrap();
    base.join(file_name).unwrap()
}

pub async fn load_string(file_name: &str) -> Result<String, ()> {
    #[cfg(target_arch = "wasm32")]
    {
        let url = format_url(file_name);
        Ok(reqwest::get(url)
            .await
            .map_err(|_| ())?
            .text()
            .await
            .map_err(|_| ()))?
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::env::current_dir;
        let path = std::path::Path::new(current_dir().unwrap().to_str().unwrap())
            .join("res")
            .join(file_name);
        std::fs::read_to_string(path).map_err(|_| ())
    }
}

pub async fn load_binary(file_name: &str) -> Result<Vec<u8>, ()> {
    #[cfg(target_arch = "wasm32")]
    {
        let url = format_url(file_name);
        Ok(reqwest::get(url)
            .await
            .map_err(|_| ())?
            .bytes()
            .await
            .map_err(|_| ())?
            .to_vec())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::env::current_dir;
        let path = std::path::Path::new(current_dir().unwrap().to_str().unwrap())
            .join("res")
            .join(file_name);
        std::fs::read(path).map_err(|_| ())
    }
}
