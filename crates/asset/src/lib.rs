use app::app::Schedule;
use app::{
    app::{App, AppSchedule},
    plugins::Plugin,
};
use crossbeam_channel::{Sender, TryRecvError};
use ecs::{DumbVec, EventReader, EventWriter, Res, ResMut, SparseArray, WinnyEvent, WinnyResource};
use prelude::handle::{ErasedHandle, Handle};
use prelude::server::{AssetHandleCreator, AssetServer};
use prelude::AssetId;
use reader::ByteReader;
use std::path::PathBuf;
use std::sync::Arc;
use std::{
    fmt::{Debug, Display},
    io::{BufReader, Cursor},
};
use util::tracing::{error, info};

pub mod handle;
pub mod prelude;
pub mod reader;
pub mod server;
pub mod toml;
pub mod watcher;

pub struct AssetLoaderPlugin;

impl Plugin for AssetLoaderPlugin {
    fn build(&mut self, app: &mut App) {
        app.insert_resource(AssetServer::default())
            .register_event::<ReloadAsset>()
            .add_systems(Schedule::PostUpdate, reload_assets);
    }
}

/// Event to automatically reload an [`Asset`] loaded by the [`AssetServer`].
///
/// Emitted by a [`watcher::FileWatcherBundle`] or [`watcher::DirWatcherBundle`] spawned with the marker struct
/// [`watcher::WatchForAsset`].
#[derive(WinnyEvent, Debug)]
pub struct ReloadAsset(PathBuf);

fn reload_assets(server: Res<AssetServer>, reader: EventReader<ReloadAsset>) {
    for event in reader.read() {
        server.reload(&event.0);
    }
}

pub trait Asset: Send + Sync + 'static {}

/// Collection of [`Asset`]s.
///
/// Created by [`AssetApp::register_asset_loader`].
#[derive(WinnyResource)]
pub struct Assets<A: Asset> {
    storage: SparseArray<AssetId, A>,
    handler: Arc<AssetHandleCreator>,
}

impl<A: Asset> Default for Assets<A> {
    fn default() -> Self {
        Self {
            storage: SparseArray::default(),
            handler: Arc::new(AssetHandleCreator::default()),
        }
    }
}

impl<A: Asset> Debug for Assets<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Assets").finish_non_exhaustive()
    }
}

impl<A: Asset> Assets<A> {
    pub(crate) fn insert(&mut self, asset: A, id: AssetId) {
        self.storage.insert(id.0 as usize, asset);
    }

    pub fn get(&self, handle: &Handle<A>) -> Option<&A> {
        self.storage.get(&handle.id())
    }

    pub fn get_mut(&mut self, handle: &Handle<A>) -> Option<&mut A> {
        self.storage.get_mut(&handle.id())
    }

    pub fn remove(&mut self, handle: &Handle<A>) -> A {
        self.handler.remove(handle.id());
        self.storage.take(handle.id().0 as usize).unwrap()
    }

    pub fn add(&mut self, asset: A) -> Handle<A> {
        let handle = self.handler.reserve();
        self.insert(asset, handle.id());

        handle.into()
    }
}

pub trait AssetApp {
    fn register_asset<A: Asset>(&mut self) -> &mut Self;
    fn register_asset_loader<A: Asset>(&mut self, loader: impl AssetLoader) -> &mut Self;
}

impl AssetApp for App {
    fn register_asset<A: Asset>(&mut self) -> &mut Self {
        let assets: Assets<A> = Assets::default();
        self.insert_resource(assets)
            .register_event::<AssetLoaderEvent<A>>();

        self
    }

    fn register_asset_loader<A: Asset>(&mut self, loader: impl AssetLoader) -> &mut Self {
        let (asset_result_tx, asset_result_rx) = crossbeam_channel::unbounded();

        self.add_systems(Schedule::PostUpdate, flush_errored_handles::<A>);

        self.add_systems(
            AppSchedule::Platform,
            move |mut assets: ResMut<Assets<A>>,
                  mut asset_loader_events: EventWriter<AssetLoaderEvent<A>>| {
                match asset_result_rx.try_recv() {
                    Ok(event) => match event {
                        AssetEvent::Err {
                            error,
                            path,
                            handle,
                        } => asset_loader_events.send(AssetLoaderEvent::Err {
                            error,
                            path,
                            handle: handle.into(),
                        }),
                        AssetEvent::Loaded {
                            path,
                            handle,
                            asset,
                        } => {
                            info!("Loaded asset [{}]: {:?}", std::any::type_name::<A>(), path);

                            asset_loader_events.send(AssetLoaderEvent::Loaded {
                                handle: handle.into(),
                            });

                            assets.insert(asset.into_asset(), handle.id());
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

        {
            let handler = self.world().resource::<Assets<A>>().handler.clone();
            let server = self.world_mut().resource_mut::<AssetServer>();
            server.register_loader::<A>(loader, asset_result_tx, handler);
        }

        self
    }
}

fn flush_errored_handles<A: Asset>(
    reader: EventReader<AssetLoaderEvent<A>>,
    server: Res<AssetServer>,
) {
    for event in reader.peak_read() {
        match event {
            AssetLoaderEvent::Err { path, .. } => server.remove::<A, &String>(path),
            _ => (),
        }
    }
}

#[derive(WinnyEvent)]
pub enum AssetLoaderEvent<A: Asset> {
    Loaded {
        handle: Handle<A>,
    },
    Err {
        handle: Handle<A>,
        path: String,
        error: AssetLoaderError,
    },
}

/// Temporary [`Asset`] storage.
struct ErasedAsset {
    asset: DumbVec,
}

impl<A: Asset> From<A> for ErasedAsset {
    fn from(value: A) -> Self {
        Self {
            asset: {
                let mut storage = DumbVec::new::<A>();
                unsafe { storage.push(value) };

                storage
            },
        }
    }
}

impl ErasedAsset {
    /// Caller ensures that this contains [`Asset`] of type A.
    fn into_asset<A: Asset>(mut self) -> A {
        unsafe { self.asset.pop::<A>() }
    }
}

enum AssetEvent {
    Loaded {
        path: PathBuf,
        handle: ErasedHandle,
        asset: ErasedAsset,
    },
    Err {
        handle: ErasedHandle,
        path: String,
        error: AssetLoaderError,
    },
}

pub trait AssetLoader: Send + Sync + 'static {
    type Asset: Asset;
    type Settings: Default + Send + Sync;

    #[allow(async_fn_in_trait)]
    async fn load(
        reader: ByteReader<Cursor<Vec<u8>>>,
        settings: Self::Settings,
        path: String,
        ext: &str,
    ) -> Result<Self::Asset, AssetLoaderError>;
    fn extensions(&self) -> &'static [&'static str];
    fn settings(&self) -> Self::Settings {
        Self::Settings::default()
    }
}

trait ErasedAssetLoader: Send + Sync + 'static {
    fn load(
        &self,
        handler: &AssetHandleCreator,
        sender: Sender<AssetEvent>,
        path: String,
        ext: String,
    ) -> ErasedHandle;
}

#[cfg(not(target_arch = "wasm32"))]
impl<L: AssetLoader> ErasedAssetLoader for L {
    fn load(
        &self,
        handler: &AssetHandleCreator,
        sender: Sender<AssetEvent>,
        path: String,
        ext: String,
    ) -> ErasedHandle {
        let handle = handler.reserve();

        let settings = self.settings();
        std::thread::spawn(move || {
            let binary = match pollster::block_on(load_binary(path.as_str())) {
                Ok(f) => f,
                Err(_) => {
                    error!("Could not find file: {:?}", path);
                    if let Err(e) = sender.send(AssetEvent::Err {
                        handle,
                        path,
                        error: AssetLoaderError::FileNotFound,
                    }) {
                        error!("Asset sender error: {}", e);
                    }

                    return;
                }
            };
            let reader = ByteReader::new(BufReader::new(Cursor::new(binary)));

            let result = pollster::block_on(L::load(reader, settings, path.clone(), ext.as_str()));
            if let Err(e) = sender.send(match result {
                Ok(a) => AssetEvent::Loaded {
                    path: path.into(),
                    handle: handle.clone(),
                    asset: a.into(),
                },
                Err(e) => AssetEvent::Err {
                    error: e,
                    handle,
                    path,
                },
            }) {
                error!("Asset sender error: {}", e);
            }
        });

        handle
    }
}

#[cfg(target_arch = "wasm32")]
impl<L: AssetLoader> ErasedAssetLoader for L {
    fn load(
        &self,
        handler: &AssetHandleCreator,
        sender: Sender<AssetEvent>,
        path: String,
        ext: String,
    ) -> ErasedHandle {
        let handle = handler.reserve();

        let settings = self.settings();
        wasm_bindgen_futures::spawn_local(async move {
            let binary = load_binary(path.as_str()).await.unwrap();
            let reader = ByteReader::new(BufReader::new(Cursor::new(binary)));
            let result = L::load(reader, settings, path.clone(), ext.as_str())
                .await;
            if let Err(e) = sender.send(match result {
                Ok(a) => AssetEvent::Loaded {
                    path: path.into(),
                    handle: handle.clone(),
                    asset: a.into(),
                },
                Err(e) => AssetEvent::Err {
                    error: e,
                    handle,
                    path,
                },
            }) {
                error!("Asset sender error: {}", e);
            }
        });

        handle
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
        let path = std::path::Path::new(current_dir().unwrap().to_str().unwrap()).join(file_name);
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
        let path = std::path::Path::new(current_dir().unwrap().to_str().unwrap()).join(file_name);
        std::fs::read(path).map_err(|_| ())
    }
}

#[derive(Debug)]
pub enum AssetLoaderError {
    UnsupportedFileExtension,
    FileNotFound,
    FailedToParse,
    FailedToBuild,
    SyntaxError,
    SemanticError,
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
            Self::SyntaxError => {
                write!(f, "Syntax error in file type")
            }
            Self::SemanticError => {
                write!(f, "Semantic error in file type")
            }
        }
    }
}

impl std::error::Error for AssetLoaderError {}
