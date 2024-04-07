use std::{
    ffi::OsString,
    marker::PhantomData,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use libloading::{Error, Symbol};
use logging::{error, trace};

use crate::{Commands, Query, World};

pub type StoredSystem = Box<dyn System>;
pub type SystemSet = Vec<StoredSystem>;

pub struct Scheduler {
    startup: Vec<SystemSet>,
    pre_update: Vec<SystemSet>,
    update: Vec<SystemSet>,
    post_update: Vec<SystemSet>,
    exit: Vec<SystemSet>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            startup: vec![],
            pre_update: vec![],
            update: vec![],
            post_update: vec![],
            exit: vec![],
        }
    }

    pub fn add_systems<M, B: SystemBundle<M>>(&mut self, schedule: Schedule, systems: B) {
        let storage = match schedule {
            Schedule::StartUp => &mut self.startup,
            Schedule::PreUpdate => &mut self.pre_update,
            Schedule::Update => &mut self.update,
            Schedule::PostUpdate => &mut self.post_update,
            Schedule::Exit => &mut self.exit,
        };

        storage.push(systems.into_set());
    }

    fn run_schedule(&mut self, schedule: Schedule, world: &mut World) {
        let storage = match schedule {
            Schedule::StartUp => &mut self.startup,
            Schedule::PreUpdate => &mut self.pre_update,
            Schedule::Update => &mut self.update,
            Schedule::PostUpdate => &mut self.post_update,
            Schedule::Exit => &mut self.exit,
        };

        for set in storage.iter_mut() {
            for system in set.iter_mut() {
                system.run(world);
            }
        }
    }

    pub fn startup(&mut self, world: &mut World) {
        self.run_schedule(Schedule::StartUp, world);
    }

    pub fn run(&mut self, world: &mut World) {
        self.run_schedule(Schedule::PreUpdate, world);
        self.run_schedule(Schedule::Update, world);
        self.run_schedule(Schedule::PostUpdate, world);
    }

    pub fn exit(&mut self, world: &mut World) {
        self.run_schedule(Schedule::Exit, world);
    }
}

pub enum Schedule {
    StartUp,
    PreUpdate,
    Update,
    PostUpdate,
    Exit,
}

pub trait SystemBundle<Marker> {
    fn into_set(self) -> Vec<StoredSystem>;
}

impl<F: 'static, P> SystemBundle<P> for F
where
    F: IntoSystem<P>,
{
    fn into_set(self) -> Vec<StoredSystem> {
        vec![Box::new(self.into_system())]
    }
}

macro_rules! system_bundle_impl {
    ($(($t:ident, $p:ident))*) => {
        #[allow(non_snake_case)]
        impl<$($t: 'static, $p),*> SystemBundle<($($p,)*)> for ($($t,)*)
            where
                $($t: IntoSystem<$p>),*
        {
            fn into_set(self) -> Vec<StoredSystem>
            {
               let ($($t,)*) = self;
               vec![
                $(
                    Box::new($t.into_system()),
                )*
               ]
            }
        }
    };

    ($(($t:ident, $p:ident)),*, $next:ident) => {
        system_bundle_impl!($(($t, $p)),*);
        system_bundle_impl!($(($t, $p)),*, $next);
    }
}

system_bundle_impl!((A, B));
system_bundle_impl!((A, B)(C, D));
system_bundle_impl!((A, B)(C, D)(E, F));
system_bundle_impl!((A, B)(C, D)(E, F)(G, H));
system_bundle_impl!((A, B)(C, D)(E, F)(G, H)(I, J));
system_bundle_impl!((A, B)(C, D)(E, F)(G, H)(I, J)(K, L));
system_bundle_impl!((A, B)(C, D)(E, F)(G, H)(I, J)(K, L)(M, N));
system_bundle_impl!((A, B)(C, D)(E, F)(G, H)(I, J)(K, L)(M, N)(O, P));

trait SystemParam {
    type Item<'new>;

    fn to_param<'p>(world: &'p World) -> Self::Item<'p>;
}

impl SystemParam for Commands {
    type Item<'new> = Commands;

    fn to_param(_world: &World) -> Self::Item<'_> {
        Commands::new()
    }
}

impl<'w, T, F> SystemParam for Query<'w, T, F> {
    type Item<'new> = Query<'new, T, F>;

    fn to_param(world: &World) -> Self::Item<'_> {
        Query::new(world)
    }
}

pub trait System: 'static + Send {
    fn run(&mut self, world: &mut World);
}

pub struct SystemFunc<Input, F> {
    f: F,
    name: &'static str,
    _phantom: PhantomData<fn() -> Input>,
}

trait IntoSystem<Input> {
    type System: System + Send;

    fn into_system(self) -> Self::System;
}

macro_rules! impl_system {
    (
        $($params:ident),*
    ) => {
        #[allow(non_snake_case)]
        impl<F: 'static, $($params: 'static + SystemParam),*> System for SystemFunc<($($params,)*), F>
            where
                for<'a, 'b> &'a mut F:
                    FnMut( $($params),* ) +
                    FnMut( $(<$params as SystemParam>::Item<'b>),* ),
                    F: Send
        {
            fn run(&mut self, world: &mut World) {
                fn call_inner<$($params),*>(
                    mut f: impl FnMut($($params),*),
                    $($params: $params),*
                ) {
                    f($($params),*)
                }

                $(
                    let $params = $params::to_param(world);
                )*

                call_inner(&mut self.f, $($params),*);
            }
        }
    }
}

impl_system!(A);
impl_system!(A, B);
impl_system!(A, B, C);
impl_system!(A, B, C, D);
impl_system!(A, B, C, D, E);
impl_system!(A, B, C, D, E, G);
impl_system!(A, B, C, D, E, G, H);
impl_system!(A, B, C, D, E, G, H, I);
impl_system!(A, B, C, D, E, G, H, I, J);
impl_system!(A, B, C, D, E, G, H, I, J, K);

macro_rules! impl_system_param {
    (
        $($params:ident),*
    ) => {
        impl<$($params: SystemParam),*> SystemParam for ($($params,)*) {
            type Item<'new> = ($($params::Item<'new>,)*);

            fn to_param(world: &World) -> Self::Item<'_> {
                (
                    $($params::to_param(world),)*
                )
            }
        }
    }
}

impl_system_param!(A);
impl_system_param!(A, B);
impl_system_param!(A, B, C);
impl_system_param!(A, B, C, D);
impl_system_param!(A, B, C, D, E);
impl_system_param!(A, B, C, D, E, G);
impl_system_param!(A, B, C, D, E, G, H);
impl_system_param!(A, B, C, D, E, G, H, I);
impl_system_param!(A, B, C, D, E, G, H, I, J);
impl_system_param!(A, B, C, D, E, G, H, I, J, K);

macro_rules! impl_into_system {
    (
        $($params:ident),*
    ) => {
        impl<F: 'static, $($params: 'static + SystemParam),*> IntoSystem<($($params,)*)> for F
            where
                for<'a, 'b> &'a mut F:
                    FnMut( $($params),* ) +
                    FnMut( $(<$params as SystemParam>::Item<'b>),* ),
                    F: Send
        {
            type System = SystemFunc<($($params,)*), Self>;

            fn into_system(self) -> Self::System {
                SystemFunc {
                    f: self,
                    name: stringify!(self),
                    _phantom: Default::default(),
                }
            }
        }
    }
}

impl_into_system!(A);
impl_into_system!(A, B);
impl_into_system!(A, B, C);
impl_into_system!(A, B, C, D);
impl_into_system!(A, B, C, D, E);
impl_into_system!(A, B, C, D, E, G);
impl_into_system!(A, B, C, D, E, G, H);
impl_into_system!(A, B, C, D, E, G, H, I);
impl_into_system!(A, B, C, D, E, G, H, I, J);
impl_into_system!(A, B, C, D, E, G, H, I, J, K);

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
