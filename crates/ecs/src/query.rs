use std::{
    borrow::BorrowMut,
    cell::{Ref, RefMut},
    marker::PhantomData,
};

use itertools::Itertools;

use crate::{world, Archetype, Component, Entity, TypeGetter, TypeId, World, ENTITY};

pub trait Filter {
    fn condition(arch: &Archetype) -> bool;
}

pub struct With<T>(PhantomData<T>);
pub struct Without<T>(PhantomData<T>);
pub struct Or<T>(PhantomData<T>);

impl<T: TypeGetter> Filter for With<T> {
    fn condition(arch: &Archetype) -> bool {
        arch.component_storage.ids.contains::<T>()
    }
}

impl<T: TypeGetter> Filter for Without<T> {
    fn condition(arch: &Archetype) -> bool {
        !arch.component_storage.ids.contains::<T>()
    }
}

impl<T: TypeGetter> Filter for Or<T> {
    fn condition(arch: &Archetype) -> bool {
        arch.component_storage.ids.contains::<T>()
    }
}

impl Filter for () {
    fn condition(_: &Archetype) -> bool {
        true
    }
}

macro_rules! or_expand {
    ($($t:ident)*) => {
        impl<$($t: Filter),*> Filter for Or<($($t,)*)> {
            fn condition(arch: &Archetype) -> bool {
                $($t::condition(arch))||*
            }
        }
    };

    ($(($t:ident)),*, $next:ident) => {
        filter_expand!($(($t)),*);
        filter_expand!($(($t)),*, $next);
    }
}

or_expand!(A);
or_expand!(A B);
or_expand!(A B C);
or_expand!(A B C D);
or_expand!(A B C D E);
or_expand!(A B C D E F);
or_expand!(A B C D E F G);
or_expand!(A B C D E F G H);
or_expand!(A B C D E F G H I);
or_expand!(A B C D E F G H I J);
or_expand!(A B C D E F G H I J K);
or_expand!(A B C D E F G H I J K L);

macro_rules! filter_expand {
    ($($t:ident)*) => {
        impl<$($t: Filter),*> Filter for ($($t,)*) {
            fn condition(arch: &Archetype) -> bool {
                $($t::condition(arch))&&*
            }
        }
    };

    ($(($t:ident)),*, $next:ident) => {
        filter_expand!($(($t)),*);
        filter_expand!($(($t)),*, $next);
    }
}

filter_expand!(A);
filter_expand!(A B);
filter_expand!(A B C);
filter_expand!(A B C D);
filter_expand!(A B C D E);
filter_expand!(A B C D E F);
filter_expand!(A B C D E F G);
filter_expand!(A B C D E F G H);
filter_expand!(A B C D E F G H I);
filter_expand!(A B C D E F G H I J);
filter_expand!(A B C D E F G H I J K);
filter_expand!(A B C D E F G H I J K L);

pub struct Query<'a, T, F = ()> {
    world: &'a World,
    query: PhantomData<T>,
    filter: PhantomData<F>,
}

impl<'a, T, F> Query<'a, T, F> {
    pub fn new(world: &'a World) -> Self {
        Self {
            world,
            query: PhantomData,
            filter: PhantomData,
        }
    }
}

// trait SystemParam {
//     type Output;
//
//     fn into_param(&self, world: &World) -> Self::Output;
// }
//
// trait IntoSystem {
//     fn into_system(self) -> Box<dyn System>;
// }
//
//
// trait System {
//     fn run(&mut self, world: &World);
// }
//
//
// impl<T: WorldQuery, U: Fn(T,)> System for U {
//     fn run(&mut self, world: &World) {
//         (self)(T::into_param(&self, world));
//     }
// }
//
//
// impl<'a, T: SystemParam> IntoSystem for dyn Fn(T,) {
//     fn into_system(self) -> Box<dyn System> {
//         Box::new(move |world: &World| (self)(T::into_param(&self, world)))
//     }
// }

pub trait WorldQuery {
    type Output;

    fn iter(&self) -> impl Iterator<Item = Self::Output>;
    fn get(&self, id: &Entity) -> Result<Self::Output, ()>;
    fn get_single(&self) -> Result<Self::Output, ()>;
}

pub trait WorldQueryMut {
    type Output;

    fn iter_mut(&mut self) -> impl Iterator<Item = Self::Output>;
    fn get_mut(&self, id: &Entity) -> Result<Self::Output, ()>;
    fn get_single_mut(&self) -> Result<Self::Output, ()>;
}

fn map_vec<'a, T: TypeGetter + Component>(
    archetype: &'a Archetype,
) -> impl Iterator<Item = Ref<'a, T>> {
    let len = archetype.len();

    (0..len)
        .map(|i| {
            let vec = match archetype.borrow_component_vec::<T>().ok_or(()) {
                Ok(vec) => vec,
                Err(()) => return Err(()),
            };

            Ok(Ref::map(vec, |v| &v[i]))
        })
        .filter(|res| {
            if res.is_err() {
                println!("Query Error for: {:?}", T::type_name());
                return false;
            }
            true
        })
        .map(|res| res.unwrap())
}

fn map_vec_mut<'a, T: TypeGetter + Component>(
    archetype: &'a Archetype,
) -> impl Iterator<Item = RefMut<'a, T>> {
    let len = archetype.len();

    (0..len)
        .map(|i| {
            let vec = match archetype.borrow_mut_component_vec::<T>().ok_or(()) {
                Ok(vec) => vec,
                Err(()) => return Err(()),
            };

            Ok(RefMut::map(vec, |v| &mut v[i]))
        })
        .filter(|res| {
            if res.is_err() {
                println!("Query Error for: {:?}", T::type_name());
                return false;
            }
            true
        })
        .map(|res| res.unwrap())
}

impl<'b, T: TypeGetter + Component, F: Filter> WorldQuery for Query<'b, T, F> {
    type Output = Ref<'b, T>;

    fn iter(&self) -> impl Iterator<Item = Self::Output> {
        self.world
            .archetypes
            .iter()
            .filter(|arch| arch.component_storage.ids.contains::<T>())
            .filter(|arch| F::condition(arch))
            .map(|arch| map_vec::<T>(arch))
            .flatten()
    }

    fn get(&self, id: &Entity) -> Result<Self::Output, ()> {
        let len = self.world.archetypes[id.archetype_index()].len();

        (0..len)
            .map(|_| map_vec::<T>(&self.world.archetypes[id.archetype_index()]))
            .flatten()
            .nth(id.storage_index())
            .ok_or(())
    }

    fn get_single(&self) -> Result<Self::Output, ()> {
        self.world
            .archetypes
            .iter()
            .filter(|arch| arch.component_storage.ids.contains::<T>())
            .filter(|arch| F::condition(arch))
            .map(|arch| map_vec::<T>(arch))
            .flatten()
            .exactly_one()
            .map_err(|_| ())
    }
}

impl<'b, T: TypeGetter + Component, F: Filter> WorldQueryMut for Query<'b, T, F> {
    type Output = RefMut<'b, T>;

    fn iter_mut(&mut self) -> impl Iterator<Item = Self::Output> {
        self.world
            .archetypes
            .iter()
            .filter(|arch| arch.component_storage.ids.contains::<T>())
            .filter(|arch| F::condition(arch))
            .map(|arch| map_vec_mut::<T>(arch))
            .flatten()
    }

    fn get_mut(&self, id: &Entity) -> Result<Self::Output, ()> {
        let len = self.world.archetypes[id.archetype_index()].len();

        (0..len)
            .map(|_| map_vec_mut::<T>(&self.world.archetypes[id.archetype_index()]))
            .flatten()
            .nth(id.storage_index())
            .ok_or(())
    }

    fn get_single_mut(&self) -> Result<Self::Output, ()> {
        self.world
            .archetypes
            .iter()
            .filter(|arch| arch.component_storage.ids.contains::<T>())
            .filter(|arch| F::condition(arch))
            .map(|arch| map_vec_mut::<T>(arch))
            .flatten()
            .exactly_one()
            .map_err(|_| ())
    }
}

macro_rules! queries {
      ($($t:ident)*) => {
         #[allow(non_snake_case)]
          impl<'b, $($t: TypeGetter + Component),*, Fil: Filter> WorldQuery for Query<'b, ($($t,)*), Fil> {
            type Output = ($(Ref<'b, $t>,)*);

            fn iter(&self) -> impl Iterator<Item = Self::Output> {
             self.world.archetypes
                .iter()
                .filter(|arch| $(arch.component_storage.ids.contains::<$t>())&&*)
                .filter(|arch| Fil::condition(arch))
                .map(|arch| {
                    itertools::multizip((
                        $(
                            {
                                map_vec::<$t>(arch)
                            },
                        )*
                    ))

                })
                .flatten()
            }


            fn get(&self, id: &Entity) -> Result<Self::Output, ()> {
                let len = self.world.archetypes[id.archetype_index()].len();

                (0..len)
                    .map(|_| itertools::multizip(($({
                                map_vec::<$t>(&self.world.archetypes[id.archetype_index()])
                            },)*))
                    )
                    .flatten()
                    .nth(id.storage_index())
                    .ok_or(())
            }


            fn get_single(&self) -> Result<Self::Output, ()> {
                self.world
                    .archetypes
                    .iter()
                    .filter(|arch| $(arch.component_storage.ids.contains::<$t>())&&*)
                    .filter(|arch| Fil::condition(arch))
                    .map(|arch| {
                        itertools::multizip((
                                $(
                                    {
                                        map_vec::<$t>(arch)
                                    },
                                    )*
                                ))
                    })
                    .flatten()
                    .exactly_one()
                    .map_err(|_| ())
            }
          }

          impl<'b, $($t: TypeGetter + Component),*, Fil: Filter> WorldQueryMut for Query<'b, ($($t,)*), Fil> {
              type Output = ($(RefMut<'b, $t>,)*);

            fn iter_mut(&mut self) -> impl Iterator<Item = Self::Output> {
             self.world.archetypes
                .iter()
                .filter(|arch| $(arch.component_storage.ids.contains::<$t>())&&*)
                .filter(|arch| Fil::condition(arch))
                .map(|arch| {
                    itertools::multizip((
                        $(
                            {
                                map_vec_mut::<$t>(arch)
                            },
                        )*
                    ))

                })
                .flatten()
            }


            fn get_mut(&self, id: &Entity) -> Result<Self::Output, ()> {
                let len = self.world.archetypes[id.archetype_index()].len();

                (0..len)
                    .map(|_| itertools::multizip(($({
                                map_vec_mut::<$t>(&self.world.archetypes[id.archetype_index()])
                            },)*))
                    )
                    .flatten()
                    .nth(id.storage_index())
                    .ok_or(())
            }

            fn get_single_mut(&self) -> Result<Self::Output, ()> {
                self.world
                    .archetypes
                    .iter()
                    .filter(|arch| $(arch.component_storage.ids.contains::<$t>())&&*)
                    .filter(|arch| Fil::condition(arch))
                    .map(|arch| {
                        itertools::multizip((
                                $(
                                    {
                                        map_vec_mut::<$t>(arch)
                                    },
                                    )*
                                ))
                    })
                    .flatten()
                    .exactly_one()
                    .map_err(|_| ())
            }
          }
      };

      ($(($t:ident)),*, $next:ident) => {
          queries!($(($t)),*);
          queries!($(($t)),*, $next);
      }
 }

queries!(A);
queries!(A B);
queries!(A B C);
queries!(A B C D);
queries!(A B C D E);
queries!(A B C D E F);
queries!(A B C D E F G);
queries!(A B C D E F G H);
queries!(A B C D E F G H I);
queries!(A B C D E F G H I J);
queries!(A B C D E F G H I J K);
queries!(A B C D E F G H I J K L);
