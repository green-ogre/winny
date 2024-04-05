use std::{
    borrow::{Borrow, BorrowMut},
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    marker::PhantomData,
};

use itertools::*;
use logging::{error, trace};

use crate::{
    entity::Entity, world, Archetype, Component, EntityMeta, Storage, StorageType, Table,
    TypeGetter, TypeId, World,
};

use crate::storage::*;

pub trait Filter {
    fn condition(arch: &Archetype) -> bool;
}

pub struct With<T>(PhantomData<T>);
pub struct Without<T>(PhantomData<T>);
pub struct Or<T>(PhantomData<T>);

impl<T: TypeGetter> Filter for With<T> {
    fn condition(arch: &Archetype) -> bool {
        arch.contains::<T>()
    }
}

impl<T: TypeGetter> Filter for Without<T> {
    fn condition(arch: &Archetype) -> bool {
        !arch.contains::<T>()
    }
}

impl<T: TypeGetter> Filter for Or<T> {
    fn condition(arch: &Archetype) -> bool {
        arch.contains::<T>()
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
    pub world: &'a World,
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

pub struct EntityQuery<'a, T, F = ()> {
    world: &'a World,
    query: PhantomData<T>,
    filter: PhantomData<F>,
}

impl<'a, T, F> EntityQuery<'a, T, F> {
    pub fn new(world: &'a World) -> Self {
        Self {
            world,
            query: PhantomData,
            filter: PhantomData,
        }
    }
}

pub trait WorldQuery {
    type Output;

    fn iter(&self) -> impl Iterator<Item = Self::Output>;
    fn get(&self, id: Entity) -> Result<Self::Output, ()>;
    fn get_single(&self) -> Result<Self::Output, ()>;
}

pub trait WorldQueryMut {
    type Output;

    fn iter_mut(&mut self) -> impl Iterator<Item = Self::Output>;
    fn get_mut(&self, id: Entity) -> Result<Self::Output, ()>;
    fn get_single_mut(&self) -> Result<Self::Output, ()>;
}

fn log_failed_query(
    archetype: &Archetype,
    table: &Table,
    type_name: String,
    type_id: TypeId,
    storage_type: StorageType,
) {
    let buf = format!(
        "Type Name: {}, Type Id: {:?}, Storage Type: {:?},\n\n{:#?}, {:#?}",
        type_name, type_id, storage_type, archetype, table
    );
    std::fs::write("temp/invalid_query.txt", &buf).unwrap()
}

fn map_vec<'a, T: Storage + TypeGetter + Component>(
    archetype: &'a Archetype,
    table: &'a Table,
) -> impl Iterator<Item = Ref<'a, T>> {
    let len = archetype.entities.len();

    (0..len).map(|i| match T::storage_type() {
        StorageType::SparseSet => todo!(),
        StorageType::Table => {
            let vec = table.borrow_component_vec::<T>().unwrap_or_else(|| {
                log_failed_query(
                    archetype,
                    table,
                    T::type_name().as_string(),
                    T::type_id(),
                    T::storage_type(),
                );
                panic!("Logged failed query");
            });

            Ref::map(vec, |v| &v[archetype.entities[i].1 .0])
        }
    })
}

fn map_vec_mut<'a, T: Storage + TypeGetter + Component>(
    archetype: &'a Archetype,
    table: &'a Table,
) -> impl Iterator<Item = RefMut<'a, T>> {
    let len = archetype.entities.len();

    (0..len).map(|i| match T::storage_type() {
        StorageType::SparseSet => todo!(),
        StorageType::Table => {
            let vec = table.borrow_mut_component_vec::<T>().unwrap_or_else(|| {
                log_failed_query(
                    archetype,
                    table,
                    T::type_name().as_string(),
                    T::type_id(),
                    T::storage_type(),
                );
                panic!("Logged failed query");
            });
            RefMut::map(vec, |v| &mut v[archetype.entities[i].1 .0])
        }
    })
}

impl<'b, T: TypeGetter + Component + Storage, F: Filter> WorldQuery for Query<'b, T, F> {
    type Output = Ref<'b, T>;

    fn iter(&self) -> impl Iterator<Item = Self::Output> {
        self.world
            .archetypes
            .iter()
            .filter(|arch| arch.contains::<T>())
            .filter(|arch| F::condition(arch))
            .map(|arch| map_vec::<T>(arch, &self.world.tables[arch.table_id]))
            .flatten()
    }

    fn get(&self, id: Entity) -> Result<Self::Output, ()> {
        let meta = self.world.get_entity(id).ok_or(())?;
        let len = self.world.archetypes[meta.location.archetype_id]
            .entities
            .len();

        let id_set = vec![T::type_id()];
        if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
            return Err(());
        }

        (0..len)
            .map(|_| {
                map_vec::<T>(
                    &self.world.archetypes[meta.location.archetype_id],
                    &self.world.tables[meta.location.table_id],
                )
            })
            .flatten()
            .nth(meta.location.table_row.0)
            .ok_or(())
    }

    fn get_single(&self) -> Result<Self::Output, ()> {
        self.world
            .archetypes
            .iter()
            .filter(|arch| arch.contains::<T>())
            .filter(|arch| F::condition(arch))
            .map(|arch| map_vec::<T>(arch, &self.world.tables[arch.table_id]))
            .flatten()
            .exactly_one()
            .map_err(|_| ())
    }
}

impl<'b, T: TypeGetter + Component + Storage, F: Filter> WorldQueryMut for Query<'b, T, F> {
    type Output = RefMut<'b, T>;

    fn iter_mut(&mut self) -> impl Iterator<Item = Self::Output> {
        self.world
            .archetypes
            .iter()
            .filter(|arch| arch.contains::<T>())
            .filter(|arch| F::condition(arch))
            .map(|arch| map_vec_mut::<T>(arch, &self.world.tables[arch.table_id]))
            .flatten()
    }

    fn get_mut(&self, id: Entity) -> Result<Self::Output, ()> {
        let meta = self.world.get_entity(id).ok_or(())?;
        let len = self.world.tables[meta.location.table_id].len;

        let id_set = vec![T::type_id()];
        if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
            return Err(());
        }

        (0..len)
            .map(|_| {
                map_vec_mut::<T>(
                    &self.world.archetypes[meta.location.archetype_id],
                    &self.world.tables[meta.location.table_id],
                )
            })
            .flatten()
            .nth(meta.location.archetype_index)
            .ok_or(())
    }

    fn get_single_mut(&self) -> Result<Self::Output, ()> {
        self.world
            .archetypes
            .iter()
            .filter(|arch| arch.contains::<T>())
            .filter(|arch| F::condition(arch))
            .map(|arch| map_vec_mut::<T>(arch, &self.world.tables[arch.table_id]))
            .flatten()
            .exactly_one()
            .map_err(|_| ())
    }
}

impl<'b, T: TypeGetter + Component + Storage, F: Filter> WorldQuery for EntityQuery<'b, T, F> {
    type Output = (Entity, Ref<'b, T>);

    fn iter(&self) -> impl Iterator<Item = Self::Output> {
        self.world
            .archetypes
            .iter()
            .filter(|arch| arch.contains::<T>())
            .filter(|arch| F::condition(arch))
            .map(|arch| {
                izip!(
                    arch.entities.iter().map(|(e, _)| e.clone()),
                    map_vec::<T>(arch, &self.world.tables[arch.table_id]),
                )
            })
            .flatten()
    }

    fn get(&self, id: Entity) -> Result<Self::Output, ()> {
        let meta = self.world.get_entity(id).ok_or(())?;
        let len = self.world.archetypes[meta.location.archetype_id]
            .entities
            .len();

        let id_set = vec![T::type_id()];
        if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
            return Err(());
        }

        (0..len)
            .map(|_| {
                izip!(
                    self.world.archetypes[meta.location.archetype_id]
                        .entities
                        .iter()
                        .map(|(e, _)| e.clone()),
                    map_vec::<T>(
                        &self.world.archetypes[meta.location.archetype_id],
                        &self.world.tables[meta.location.table_id],
                    ),
                )
            })
            .flatten()
            .nth(meta.location.table_row.0)
            .ok_or(())
    }

    fn get_single(&self) -> Result<Self::Output, ()> {
        self.world
            .archetypes
            .iter()
            .filter(|arch| arch.contains::<T>())
            .filter(|arch| F::condition(arch))
            .map(|arch| {
                izip!(
                    arch.entities.iter().map(|(e, _)| e.clone()),
                    map_vec::<T>(arch, &self.world.tables[arch.table_id]),
                )
            })
            .flatten()
            .exactly_one()
            .map_err(|_| ())
    }
}

impl<'b, T: TypeGetter + Component + Storage, F: Filter> WorldQueryMut for EntityQuery<'b, T, F> {
    type Output = (Entity, RefMut<'b, T>);

    fn iter_mut(&mut self) -> impl Iterator<Item = Self::Output> {
        self.world
            .archetypes
            .iter()
            .filter(|arch| arch.contains::<T>())
            .filter(|arch| F::condition(arch))
            .map(|arch| {
                izip!(
                    arch.entities.iter().map(|(e, _)| e.clone()),
                    map_vec_mut::<T>(arch, &self.world.tables[arch.table_id]),
                )
            })
            .flatten()
    }

    fn get_mut(&self, id: Entity) -> Result<Self::Output, ()> {
        let meta = self.world.get_entity(id).ok_or(())?;
        let len = self.world.archetypes[meta.location.archetype_id]
            .entities
            .len();

        let id_set = vec![T::type_id()];
        if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
            return Err(());
        }

        (0..len)
            .map(|_| {
                izip!(
                    self.world.archetypes[meta.location.archetype_id]
                        .entities
                        .iter()
                        .map(|(e, _)| e.clone()),
                    map_vec_mut::<T>(
                        &self.world.archetypes[meta.location.archetype_id],
                        &self.world.tables[meta.location.table_id],
                    ),
                )
            })
            .flatten()
            .nth(meta.location.archetype_index)
            .ok_or(())
    }

    fn get_single_mut(&self) -> Result<Self::Output, ()> {
        self.world
            .archetypes
            .iter()
            .filter(|arch| arch.contains::<T>())
            .filter(|arch| F::condition(arch))
            .map(|arch| {
                izip!(
                    arch.entities.iter().map(|(e, _)| e.clone()),
                    map_vec_mut::<T>(arch, &self.world.tables[arch.table_id]),
                )
            })
            .flatten()
            .exactly_one()
            .map_err(|_| ())
    }
}

macro_rules! queries {
      ($($t:ident)*) => {
         #[allow(non_snake_case)]
          impl<'b, $($t: TypeGetter + Component + Storage),*, Fil: Filter> WorldQuery for Query<'b, ($($t,)*), Fil> {
            type Output = ($(Ref<'b, $t>,)*);

            fn iter(&self) -> impl Iterator<Item = Self::Output> {
             self.world.archetypes
                .iter()
                .filter(|arch| $(arch.contains::<$t>())&&*)
                .filter(|arch| Fil::condition(arch))
                .map(|arch| {
                    izip!(
                        $(
                            {
                                map_vec::<$t>(arch, &self.world.tables[arch.table_id])
                            },
                        )*
                    )

                })
                .flatten()
            }

            fn get(&self, id: Entity) -> Result<Self::Output, ()> {
                let meta = self.world.get_entity(id).ok_or(())?;
                let len = self.world.archetypes[meta.location.archetype_id].entities.len();

                let id_set = vec![$($t::type_id(),)*];
                if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
                    return Err(());
                }

                (0..len)
                    .map(|_| izip!($({
                                map_vec::<$t>(&self.world.archetypes[meta.location.archetype_id], &self.world.tables[meta.location.table_id])
                            },)*)
                    )
                    .flatten()
                    .nth(meta.location.table_row.0)
                    .ok_or(())
            }


            fn get_single(&self) -> Result<Self::Output, ()> {
                self.world
                    .archetypes
                    .iter()
                    .filter(|arch| $(arch.contains::<$t>())&&*)
                    .filter(|arch| Fil::condition(arch))
                    .map(|arch| {
                        izip!(
                            $(
                                map_vec::<$t>(arch, &self.world.tables[arch.table_id]),
                                )*
                            )
                    })
                    .flatten()
                    .exactly_one()
                    .map_err(|_| ())
            }
          }

          impl<'b, $($t: TypeGetter + Component + Storage),*, Fil: Filter> WorldQueryMut for Query<'b, ($($t,)*), Fil> {
              type Output = ($(RefMut<'b, $t>,)*);

            fn iter_mut(&mut self) -> impl Iterator<Item = Self::Output> {
             self.world.archetypes
                .iter()
                .filter(|arch| $(arch.contains::<$t>())&&*)
                .filter(|arch| Fil::condition(arch))
                .map(|arch| {
                    izip!(
                        $(
                            map_vec_mut::<$t>(arch, &self.world.tables[arch.table_id]),
                        )*
                    )

                })
                .flatten()
            }


            fn get_mut(&self, id: Entity) -> Result<Self::Output, ()> {
                let meta = self.world.get_entity(id).ok_or(())?;
                let len = self.world.tables[meta.location.table_id].len;

                let id_set = vec![$($t::type_id(),)*];
                if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
                    return Err(());
                }

                (0..len)
                    .map(|_| izip!($(
                                map_vec_mut::<$t>(&self.world.archetypes[meta.location.archetype_id], &self.world.tables[meta.location.table_id])
                                ,)*)
                    )
                    .flatten()
                    .nth(meta.location.archetype_index)
                    .ok_or(())
            }

            fn get_single_mut(&self) -> Result<Self::Output, ()> {
                self.world
                    .archetypes
                    .iter()
                    .filter(|arch| $(arch.contains::<$t>())&&*)
                    .filter(|arch| Fil::condition(arch))
                    .map(|arch| {
                            izip!(
                                $(
                                    {
                                        map_vec_mut::<$t>(arch, &self.world.tables[arch.table_id])
                                    },
                                    )*
                                )
                    })
                    .flatten()
                    .exactly_one()
                    .map_err(|_| ())
            }
          }

         #[allow(non_snake_case)]
          impl<'b, $($t: TypeGetter + Component + Storage),*, Fil: Filter> WorldQuery for EntityQuery<'b, ($($t,)*), Fil> {
            type Output = (Entity, $(Ref<'b, $t>,)*);

            fn iter(&self) -> impl Iterator<Item = Self::Output> {
             self.world.archetypes
                .iter()
                .filter(|arch| $(arch.contains::<$t>())&&*)
                .filter(|arch| Fil::condition(arch))
                .map(|arch| {
                    izip!(
                        arch.entities.iter().map(|(e, _)| e.clone()),
                        $(
                            {
                                map_vec::<$t>(arch, &self.world.tables[arch.table_id])
                            },
                        )*
                    )

                })
                .flatten()
            }

            fn get(&self, id: Entity) -> Result<Self::Output, ()> {
                let meta = self.world.get_entity(id).ok_or(())?;
                let len = self.world.archetypes[meta.location.archetype_id].entities.len();

                let id_set = vec![$($t::type_id(),)*];
                if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
                    return Err(());
                }

                (0..len)
                    .map(|_| izip!(
                            self.world.archetypes[meta.location.archetype_id].entities.iter().map(|(e, _)| e.clone()),
                            $({
                                map_vec::<$t>(&self.world.archetypes[meta.location.archetype_id], &self.world.tables[meta.location.table_id])
                            },)*)
                    )
                    .flatten()
                    .nth(meta.location.table_row.0)
                    .ok_or(())
            }

            fn get_single(&self) -> Result<Self::Output, ()> {
                self.world
                    .archetypes
                    .iter()
                    .filter(|arch| $(arch.contains::<$t>())&&*)
                    .filter(|arch| Fil::condition(arch))
                    .map(|arch| {
                        izip!(
                            arch.entities.iter().map(|(e, _)| e.clone()),
                            $(
                                {
                                    map_vec::<$t>(arch, &self.world.tables[arch.table_id])
                                },
                                )*
                            )
                    })
                    .flatten()
                    .exactly_one()
                    .map_err(|_| ())
            }
          }

         #[allow(non_snake_case)]
          impl<'b, $($t: TypeGetter + Component + Storage),*, Fil: Filter> WorldQueryMut for EntityQuery<'b, ($($t,)*), Fil> {
            type Output = (Entity, $(RefMut<'b, $t>,)*);

            fn iter_mut(&mut self) -> impl Iterator<Item = Self::Output> {
             self.world.archetypes
                .iter()
                .filter(|arch| $(arch.contains::<$t>())&&*)
                .filter(|arch| Fil::condition(arch))
                .map(|arch| {
                    izip!(
                        arch.entities.iter().map(|(e, _)| e.clone()),
                        $(
                            {
                                map_vec_mut::<$t>(arch, &self.world.tables[arch.table_id])
                            },
                        )*
                    )

                })
                .flatten()
            }

            fn get_mut(&self, id: Entity) -> Result<Self::Output, ()> {
                let meta = self.world.get_entity(id).ok_or(())?;
                let len = self.world.tables[meta.location.table_id].len;

                let id_set = vec![$($t::type_id(),)*];
                if !self.world.archetypes[meta.location.archetype_id].contains_id_set(&id_set) {
                    return Err(());
                }

                (0..len)
                    .map(|_| izip!(
                            self.world.archetypes[meta.location.archetype_id].entities.iter().map(|(e, _)| e.clone()),
                            $({
                                map_vec_mut::<$t>(&self.world.archetypes[meta.location.archetype_id], &self.world.tables[meta.location.table_id])
                            },)*)
                    )
                    .flatten()
                    .nth(meta.location.archetype_index)
                    .ok_or(())
            }

            fn get_single_mut(&self) -> Result<Self::Output, ()> {
                self.world
                    .archetypes
                    .iter()
                    .filter(|arch| $(arch.contains::<$t>())&&*)
                    .filter(|arch| Fil::condition(arch))
                    .map(|arch| {
                        izip!(
                            arch.entities.iter().map(|(e, _)| e.clone()),
                            $(
                                {
                                    map_vec_mut::<$t>(arch, &self.world.tables[arch.table_id])
                                },
                                )*
                            )
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
