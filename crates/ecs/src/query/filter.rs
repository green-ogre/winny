use super::*;

pub trait Filter: Send + Sync {
    fn condition(arch: &Archetype) -> bool;
    fn set_access() -> Vec<ComponentAccessFilter>;
}

pub struct With<T>(PhantomData<T>);
pub struct Without<T>(PhantomData<T>);
pub struct Or<T>(PhantomData<T>);

impl<T: Component> Filter for With<T> {
    fn condition(arch: &Archetype) -> bool {
        arch.contains_type_id::<T>()
    }

    fn set_access() -> Vec<ComponentAccessFilter> {
        vec![ComponentAccessFilter::new::<T>(AccessFilter::With)]
    }
}

impl<T: Component> Filter for Without<T> {
    fn condition(arch: &Archetype) -> bool {
        !arch.contains_type_id::<T>()
    }

    fn set_access() -> Vec<ComponentAccessFilter> {
        vec![ComponentAccessFilter::new::<T>(AccessFilter::Without)]
    }
}

impl<T: Component> Filter for Or<T> {
    fn condition(arch: &Archetype) -> bool {
        arch.contains_type_id::<T>()
    }

    fn set_access() -> Vec<ComponentAccessFilter> {
        vec![ComponentAccessFilter::new::<T>(AccessFilter::Or)]
    }
}

impl Filter for () {
    fn condition(_: &Archetype) -> bool {
        true
    }

    fn set_access() -> Vec<ComponentAccessFilter> {
        vec![]
    }
}

macro_rules! or_expand {
    ($($t:ident),*) => {
        impl<$($t: Filter),*> Filter for Or<($($t,)*)> {
            fn condition(arch: &Archetype) -> bool {
                $($t::condition(arch))||*
            }

            fn set_access() -> Vec<ComponentAccessFilter> {
                let mut access = vec![];
                $(access.append(
                        &mut $t::set_access()
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

            fn set_access() -> Vec<ComponentAccessFilter> {
                let mut access = vec![];
                $(access.append(&mut $t::set_access());)*
                access
            }
        }
    }
}

all_tuples!(filter_expand, 1, 10, F);
