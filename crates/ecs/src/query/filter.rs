use crate::access::{AccessFilter, ComponentAccessFilter, SystemAccess};

use super::*;

#[cfg(not(target_arch = "wasm32"))]
pub trait Filter: Send + Sync {
    fn condition(arch: &Archetype) -> bool;
    fn system_access(components: &mut Components) -> SystemAccess;
}
#[cfg(target_arch = "wasm32")]
pub trait Filter {
    fn condition(arch: &Archetype) -> bool;
    fn system_access(components: &mut Components) -> SystemAccess;
}

pub struct With<T>(PhantomData<T>);
pub struct Without<T>(PhantomData<T>);
pub struct Or<T, O>(PhantomData<T>, PhantomData<O>);

impl<T: Component> Filter for With<T> {
    fn condition(arch: &Archetype) -> bool {
        arch.contains_type_id::<T>()
    }

    fn system_access(components: &mut Components) -> SystemAccess {
        let meta = components.register::<T>();
        SystemAccess::default().with_filter(ComponentAccessFilter::new(AccessFilter::With, *meta))
    }
}

impl<T: Component> Filter for Without<T> {
    fn condition(arch: &Archetype) -> bool {
        !arch.contains_type_id::<T>()
    }

    fn system_access(components: &mut Components) -> SystemAccess {
        let meta = components.register::<T>();
        SystemAccess::default()
            .with_filter(ComponentAccessFilter::new(AccessFilter::Without, *meta))
    }
}

impl<T: Component, O: Component> Filter for Or<T, O> {
    fn condition(arch: &Archetype) -> bool {
        arch.contains_type_id::<T>() || arch.contains_type_id::<O>()
    }

    fn system_access(components: &mut Components) -> SystemAccess {
        let meta = components.register::<T>();
        let access = SystemAccess::default()
            .with_filter(ComponentAccessFilter::new(AccessFilter::Or, *meta));
        let other_meta = components.register::<T>();
        access.with(
            SystemAccess::default()
                .with_filter(ComponentAccessFilter::new(AccessFilter::Or, *other_meta)),
        )
    }
}

impl Filter for () {
    fn condition(_: &Archetype) -> bool {
        true
    }

    fn system_access(_components: &mut Components) -> SystemAccess {
        SystemAccess::default()
    }
}

// macro_rules! or_expand {
//     ($($t:ident),*) => {
//         impl<$($t: Filter),*> Filter for Or<($($t,)*)> {
//             fn condition(arch: &Archetype) -> bool {
//                 $($t::condition(arch))||*
//             }
//
//             fn system_access(components: &mut Components) -> SystemAccess {
//                 let mut access = SystemAccess::default();
//                 $(access = access.with(
//                         $t::system_access(components)
//                         );
//                     )*
//                 access
//             }
//         }
//     }
// }
//
// all_tuples!(or_expand, 1, 10, O);

macro_rules! filter_expand {
        ($($t:ident),*) => {
        impl<$($t: Filter),*> Filter for ($($t,)*) {
            fn condition(arch: &Archetype) -> bool {
                $($t::condition(arch))&&*
            }

            fn system_access(components: &mut Components) -> SystemAccess {
                let mut access = SystemAccess::default();
                $(
                    access = access.with(
                        $t::system_access(components)
                    );
                )*
                access
            }
        }
    }
}

all_tuples!(filter_expand, 1, 10, F);

macro_rules! with_expand {
        ($($t:ident),*) => {
        impl<$($t: Component),*> Filter for With<($($t,)*)> {
            fn condition(arch: &Archetype) -> bool {
                $(arch.contains_type_id::<$t>())&&*
            }

            fn system_access(components: &mut Components) -> SystemAccess {
                let mut access = SystemAccess::default();
                access = access
                $(
                    .with_filter(
                        ComponentAccessFilter {
                            filter: AccessFilter::With,
                            meta: *components.register::<$t>(),
                        }
                    )
                )*;

                access
            }
        }
    }
}

all_tuples!(with_expand, 1, 10, F);

macro_rules! without_expand {
        ($($t:ident),*) => {
        impl<$($t: Component),*> Filter for Without<($($t,)*)> {
            fn condition(arch: &Archetype) -> bool {
                $(!arch.contains_type_id::<$t>())&&*
            }

            fn system_access(components: &mut Components) -> SystemAccess {
                let mut access = SystemAccess::default();
                access = access
                $(
                    .with_filter(
                        ComponentAccessFilter {
                            filter: AccessFilter::Without,
                            meta: *components.register::<$t>(),
                        }
                    )
                )*;

                access
            }
        }
    }
}

all_tuples!(without_expand, 1, 10, F);
