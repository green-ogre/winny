use app::prelude::*;
#[cfg(feature = "widgets")]
use ecs::egui_widget::Widget;
use ecs::{WinnyBundle, *};
use ecs::{WinnyComponent, WinnyEvent};
use std::{
    fmt::Display,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use util::info;

use crate::ReloadAsset;

#[derive(Debug)]
pub struct WatcherPlugin;

impl Plugin for WatcherPlugin {
    fn build(&mut self, app: &mut App) {
        app.register_event::<FileEvent>()
            .add_systems(AppSchedule::Platform, emit_watcher_events);
    }
}

#[derive(WinnyBundle)]
pub struct FileWatcherBundle {
    pub watcher: FileWatcher,
}

#[derive(WinnyBundle)]
pub struct DirWatcherBundle {
    pub watcher: DirWatcher,
}

/// Marker struct.
///
/// Watcher will emit [`asset::ReloadAsset`] event on change.
#[derive(WinnyComponent)]
pub struct WatchForAsset;

fn emit_watcher_events(
    mut file_writer: EventWriter<FileEvent>,
    mut asset_writer: EventWriter<ReloadAsset>,
    mut dirs: Query<(Mut<DirWatcher>, Option<WatchForAsset>)>,
    mut files: Query<(Mut<FileWatcher>, Option<WatchForAsset>)>,
) {
    for (dir, asset) in dirs.iter_mut() {
        for file in dir.watchers() {
            for change in file.changes() {
                if asset.is_some() {
                    info!("{:?}: reloading", change);
                    asset_writer.send(ReloadAsset(file.path().into()));
                }

                file_writer.send(change);
            }
        }
    }

    for (file, asset) in files.iter_mut() {
        for change in file.changes() {
            if asset.is_some() {
                info!("{:?}: reloading", change);
                asset_writer.send(ReloadAsset(file.path().into()));
            }

            file_writer.send(change);
        }
    }
}

/// Event from a [`FileState`] emitted during the [`AppSchedule::Platform`] schedule.
#[derive(WinnyEvent, Debug)]
pub enum FileEvent {
    Modified(PathBuf),
}

/// Tracks state of a file and notifies [`FileWatcher`] of any events.
#[derive(Debug)]
struct FileState {
    last_modified: SystemTime,
    path: PathBuf,
}

#[cfg(feature = "widgets")]
impl Widget for FileState {
    fn display(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("{:?}", self).as_str());
    }
}

impl FileState {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, WatcherError> {
        let metadata = match std::fs::metadata(path.as_ref()) {
            Ok(m) => m,
            Err(_) => return Err(WatcherError::FileNotFound(path.as_ref().into())),
        };
        if !metadata.is_file() {
            return Err(WatcherError::FileStateIsNotFile);
        }
        let path = path.as_ref().into();

        let last_modified = if let Ok(t) = metadata.modified() {
            t
        } else {
            SystemTime::now()
        };

        Ok(Self {
            path,
            last_modified,
        })
    }

    pub fn events(&mut self) -> Vec<FileEvent> {
        let mut events = Vec::new();
        let metadata = match std::fs::metadata(&self.path) {
            Ok(m) => m,
            Err(_) => {
                return vec![];
            }
        };

        match metadata.modified() {
            Ok(t) => {
                if t.duration_since(self.last_modified).unwrap_or_default()
                    > Duration::from_millis(10)
                {
                    events.push(FileEvent::Modified(self.path.clone()));
                    self.last_modified = t;
                }
            }
            Err(_) => (),
        }

        events
    }
}

/// Emits [`FileEvent`].
#[derive(WinnyComponent, Debug)]
pub struct FileWatcher {
    state: FileState,
}

#[cfg(feature = "widgets")]
impl Widget for FileWatcher {
    fn display(&mut self, ui: &mut ecs::egui::Ui) {
        self.state.display(ui);
    }
}

impl FileWatcher {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, WatcherError> {
        Ok(Self {
            state: FileState::new(path)?,
        })
    }

    pub fn changes(&mut self) -> Vec<FileEvent> {
        self.state.events()
    }

    pub fn path(&self) -> &Path {
        &self.state.path
    }
}

/// TODO: watch for added and removed files.
/// Collection of [`FileWatcher`]s.
#[derive(WinnyComponent, Debug)]
pub struct DirWatcher {
    path: PathBuf,
    file_watchers: Vec<FileWatcher>,
}

impl DirWatcher {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, WatcherError> {
        if !path.as_ref().is_dir() {
            return Err(WatcherError::DirIsNotDir);
        }

        fn read_dir(path: &Path, file_watchers: &mut Vec<FileWatcher>) -> Result<(), WatcherError> {
            let dir = match std::fs::read_dir(path) {
                Ok(dir) => dir,
                Err(_) => return Err(WatcherError::ReadDirectory),
            };

            for entry in dir {
                match entry {
                    Ok(entry) => {
                        let path = entry.path();
                        if path.is_dir() {
                            read_dir(&path, file_watchers)?;
                        } else {
                            file_watchers.push(FileWatcher::new(path)?);
                        }
                    }
                    Err(_) => return Err(WatcherError::Entry),
                }
            }

            Ok(())
        }

        let mut file_watchers = Vec::new();
        read_dir(path.as_ref(), &mut file_watchers)?;
        let path = path.as_ref().into();

        Ok(Self {
            path,
            file_watchers,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn watchers(&mut self) -> &mut [FileWatcher] {
        &mut self.file_watchers
    }
}

#[derive(Debug)]
pub enum WatcherError {
    Io,
    FileStateIsNotFile,
    DirIsNotDir,
    FileNotFound(PathBuf),
    Entry,
    ReadDirectory,
}

impl Display for WatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for WatcherError {}
