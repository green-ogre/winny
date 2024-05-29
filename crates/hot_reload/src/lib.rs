use std::{
    env::current_dir,
    ffi::OsString,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use app::app::App;
use app::plugins::Plugin;
use ecs::{ResMut, WinnyResource};
use logger::{error, info, trace};

pub mod prelude;

#[derive(Debug)]
pub struct Lib {
    pub lib: Option<libloading::Library>,
}

impl Lib {
    pub fn new(
        path_to_lib: &OsString,
        path_to_write: &OsString,
    ) -> Result<Self, libloading::Error> {
        if std::fs::metadata(path_to_write).is_err() {
            std::fs::write(path_to_write, "").unwrap();
            info!("File does not exist, writing new : {:?}", path_to_write);
        }

        if let Err(e) = std::fs::copy(path_to_lib, path_to_write) {
            error!("{}", e);
            panic!();
        }

        unsafe {
            Ok(Self {
                lib: Some(libloading::Library::new(path_to_write)?),
            })
        }
    }

    pub fn refresh(
        &mut self,
        path_to_lib: &OsString,
        path_to_write: &OsString,
    ) -> Result<(), libloading::Error> {
        if std::fs::metadata(path_to_write).is_err() {
            std::fs::write(path_to_write, "").unwrap();
            trace!("File does not exist, writing new : {:?}", path_to_write);
        }

        let _ = self.lib.take().unwrap().close();

        while let Err(e) = std::fs::copy(path_to_lib, path_to_write) {
            error!("{}", e);
        }

        unsafe {
            self.lib = Some(libloading::Library::new(path_to_write)?);
        }

        Ok(())
    }
}

#[derive(Debug, WinnyResource)]
pub struct LinkedLib {
    pub linked_lib: Lib,
    last_refresh: Duration,
    path_to_lib: OsString,
    path_to_write: OsString,
}

impl LinkedLib {
    pub fn new(path_to_lib: OsString, path_to_write: OsString) -> Result<Self, libloading::Error> {
        Ok(Self {
            linked_lib: Lib::new(&path_to_lib, &path_to_write)?,
            last_refresh: SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
            path_to_write,
            path_to_lib,
        })
    }

    pub fn refresh_if_modified(&mut self) {
        let Ok(last_mod) = std::fs::metadata(&self.path_to_lib) else {
            return;
        };

        let last_mod = last_mod
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap();

        if self.last_refresh >= last_mod {
            return;
        }

        info!("Reloading App...");

        self.linked_lib
            .refresh(&self.path_to_lib, &self.path_to_write)
            .unwrap();
        self.last_refresh = last_mod;
    }
}

pub struct HotReloadPlugin {
    pub crate_name: String,
}

impl Plugin for HotReloadPlugin {
    fn build(&mut self, app: &mut App) {
        if cfg!(windows) {
            let lib_path: PathBuf = [
                format!("{}", current_dir().unwrap().to_str().unwrap()),
                self.crate_name.clone(),
                "target".into(),
                #[cfg(debug_assertions)]
                "debug".into(),
                #[cfg(not(debug_assertions))]
                "release".into(),
                format!("{}.dll", self.crate_name.clone()),
            ]
            .iter()
            .collect();

            let write_path: PathBuf = [
                format!("{}", current_dir().unwrap().to_str().unwrap()),
                self.crate_name.clone(),
                "target".into(),
                #[cfg(debug_assertions)]
                "debug".into(),
                #[cfg(not(debug_assertions))]
                "release".into(),
                "libtemp.dll".into(),
            ]
            .iter()
            .collect();

            info!(
                "Path to lib: {}, Path to write: {}",
                lib_path.to_str().unwrap(),
                write_path.to_str().unwrap()
            );

            let linked_lib =
                LinkedLib::new(lib_path.into(), write_path.into()).expect("Could not find library");

            app.insert_resource(linked_lib);
            app.add_systems(ecs::Schedule::PreUpdate, (reload_if_changed,));
        }
    }
}

fn reload_if_changed(mut reload: ResMut<LinkedLib>) {
    reload.refresh_if_modified();
}
