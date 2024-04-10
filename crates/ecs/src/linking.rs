type LinkedFunc<'lib, 'w, I = ()> = libloading::Symbol<'lib, fn(I)>;

pub struct Lib {
    lib: libloading::Library,
}

impl Lib {
    pub fn new(
        path_to_lib: &OsString,
        path_to_write: &OsString,
    ) -> Result<Self, libloading::Error> {
        if std::fs::metadata(path_to_write).is_err() {
            std::fs::write(path_to_write, "").unwrap();
            trace!("File does not exist, writing new : {:?}", path_to_write);
        }

        if let Err(e) = std::fs::copy(path_to_lib, path_to_write) {
            error!("{}", e);
            panic!();
        }

        unsafe {
            Ok(Self {
                lib: libloading::Library::new(path_to_write)?,
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

        if let Err(e) = std::fs::copy(path_to_lib, path_to_write) {
            error!("{}", e);
            panic!();
        }

        unsafe {
            self.lib = libloading::Library::new(path_to_write)?;
        }

        Ok(())
    }

    // pub fn run<Input, F>(&self, f: &mut SystemFunc<Input, F>, params: Input) {
    //     unsafe {
    //         let func: Symbol<fn(Input)> = self.lib.get(f.name.as_bytes()).unwrap();
    //         func(params);
    //     }
    // }

    // pub fn get<Input>(&self, name: &'static str) -> Result<Symbol<fn('_, Input)>, Error> {
    //     unsafe { Ok(self.lib.get::<'_, fn(Input)>(name.as_bytes())?) }
    // }
}

pub struct LinkedLib {
    linked_lib: Lib,
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

    // pub fn run_system<Input, F>(&self, f: &mut SystemFunc<Input, F>, params: Input) {
    //     self.linked_lib.run(f, params);
    // }

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

        trace!("app :: Refreshing App");

        self.linked_lib
            .refresh(&self.path_to_lib, &self.path_to_write)
            .unwrap();
        self.last_refresh = last_mod;
    }
}
