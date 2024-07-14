use crate::access::{AccessFilter, ComponentAccessFilter, SystemAccess};

use super::*;

pub trait Filter: Send + Sync {
    fn condition(arch: &Archetype) -> bool;
    fn system_access(components: &mut Components) -> SystemAccess;
}

pub struct With<T>(PhantomData<T>);
pub struct Without<T>(PhantomData<T>);
pub struct Or<T>(PhantomData<T>);

impl<T: Component> Filter for With<T> {
    fn condition(arch: &Archetype) -> bool {
        arch.contains_type_id::<T>()
    }

    fn system_access(components: &mut Components) -> SystemAccess {
        let id = components.register::<T>();
        SystemAccess::default().with_filter(ComponentAccessFilter::new(AccessFilter::With, id))
    }
}

impl<T: Component> Filter for Without<T> {
    fn condition(arch: &Archetype) -> bool {
        !arch.contains_type_id::<T>()
    }

    fn system_access(components: &mut Components) -> SystemAccess {
        let id = components.register::<T>();
        SystemAccess::default().with_filter(ComponentAccessFilter::new(AccessFilter::Without, id))
    }
}

impl<T: Component> Filter for Or<T> {
    fn condition(arch: &Archetype) -> bool {
        arch.contains_type_id::<T>()
    }

    fn system_access(components: &mut Components) -> SystemAccess {
        let id = components.register::<T>();
        SystemAccess::default().with_filter(ComponentAccessFilter::new(AccessFilter::Or, id))
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

macro_rules! or_expand {
    ($($t:ident),*) => {
        impl<$($t: Filter),*> Filter for Or<($($t,)*)> {
            fn condition(arch: &Archetype) -> bool {
                $($t::condition(arch))||*
            }

            fn system_access(components: &mut Components) -> SystemAccess {
                let mut access = SystemAccess::default();
                $(access = access.with(
                        $t::system_access(components)
                        );
                    )*
                access
            }
        }
    }
}

all_tuples!(or_expand, 1, 10, O);

macro_rules! filter_expand {
        ($($t:ident),*) => {
        impl<$($t: Filter),*> Filter for ($($t,)*) {
            fn condition(arch: &Archetype) -> bool {
                $($t::condition(arch))&&*
            }

            fn system_access(components: &mut Components) -> SystemAccess {
                let mut access = SystemAccess::default();
                $(access = access.with(
                        $t::system_access(components)
                        );
                    )*
                access
            }
        }
    }
}

all_tuples!(filter_expand, 1, 10, F);
