use super::*;

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
