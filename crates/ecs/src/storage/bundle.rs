use super::*;

#[derive(Debug)]
pub enum IntoStorageError {
    ColumnMisMatch,
}

pub trait Bundle: Debug {
    fn descriptions(&self) -> Vec<ComponentDescription>;
    // fn into_storage(self) -> Vec<(TypeId, Box<dyn ComponentVec>)>;
    // fn into_storage_box(self: Box<Self>) -> Vec<Box<dyn ComponentVec>>;
    fn push_storage(self, table: &mut Table) -> Result<(), IntoStorageError>;
    // fn push_storage_box(self: Box<Self>, table: &mut Table) -> Result<(), IntoStorageError>;
    fn ids(&self) -> Vec<TypeId>;
    fn storage_locations(&self) -> Vec<StorageType>;
}

impl<T: Debug + Storage + Component + TypeGetter + 'static> Bundle for T {
    fn push_storage(self, table: &mut Table) -> Result<(), IntoStorageError> {
        table.push_column(self)?;

        Ok(())
    }

    fn descriptions(&self) -> Vec<ComponentDescription> {
        vec![ComponentDescription {
            type_id: self.type_id(),
            layout: std::alloc::Layout::new::<T>(),
            drop: new_dumb_drop::<T>(),
        }]
    }

    fn ids(&self) -> Vec<TypeId> {
        vec![self.type_id()]
    }

    fn storage_locations(&self) -> Vec<StorageType> {
        vec![self.storage_type()]
    }
}

macro_rules! bundle {
    ($($t:ident)*) => {
        #[allow(non_snake_case)]
        impl<$($t: Bundle),*> Bundle for ($($t,)*) {
            fn descriptions(&self) -> Vec<ComponentDescription> {
               let ($($t,)*) = self;

                vec![
                    $($t.descriptions(),)*
                ].into_iter().flatten().collect::<Vec<_>>()
            }

            fn push_storage(self, table: &mut Table) -> Result<(), IntoStorageError> {
               let ($($t,)*) = self;

                $(
                    assert!($t.push_storage(table).is_ok());
                )*

                Ok(())
            }

            fn ids(&self) -> Vec<TypeId>  {
               let ($($t,)*) = self;

                vec![
                    $($t.ids(),)*
                ].into_iter().flatten().collect::<Vec<_>>()
            }

            fn storage_locations(&self) -> Vec<StorageType> {
               let ($($t,)*) = self;

                vec![
                    $($t.storage_locations(),)*
                ].into_iter().flatten().collect::<Vec<_>>()
            }
        }
    };

    ($(($t:ident)),*, $next:ident) => {
        bundle!($(($t)),*);
        bundle!($(($t)),*, $next);
    }
}

bundle!(A);
bundle!(A B);
bundle!(A B C);
bundle!(A B C D);
bundle!(A B C D E);
bundle!(A B C D E F);
bundle!(A B C D E F G);
bundle!(A B C D E F G H);
bundle!(A B C D E F G H I);
bundle!(A B C D E F G H I J);
bundle!(A B C D E F G H I J K);
bundle!(A B C D E F G H I J K L);
