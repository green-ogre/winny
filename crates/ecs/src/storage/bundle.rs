use super::*;

#[derive(Debug)]
pub enum IntoStorageError {
    MismatchedShape,
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

macro_rules! bundle {
    ($($t:ident)*) => {
        #[allow(non_snake_case)]
        impl<$($t: Send + Debug + Storage + Component + TypeGetter + Clone + 'static),*> Bundle for ($($t,)*) {
            // fn into_storage(self) -> Vec<Box<dyn ComponentVec>>  {
            //    let ($($t,)*) = self;
            //     vec![
            //         $(Box::new(UnsafeCell::new(vec![$t])),)*
            //     ]
            // }

            // fn into_storage_box(self: Box<Self>) -> Vec<Box<dyn ComponentVec>> {
            //     self.into_storage()
            // }

            fn descriptions(&self) -> Vec<ComponentDescription> {
                vec![
                    $(ComponentDescription {
                        type_id: TypeId::of::<$t>(),
                        layout: std::alloc::Layout::new::<$t>(),
                        drop: new_dumb_drop::<$t>()
                    },)*
                ]
            }

            fn push_storage(self, table: &mut Table) -> Result<(), IntoStorageError> {
               let ($($t,)*) = self;

                $(
                    assert!(table.push_column($t).is_ok());
                )*

                Ok(())
            }

            // fn push_storage_box(self: Box<Self>, table: &mut Table) -> Result<(), IntoStorageError> {
            //     self.push_storage(table)
            // }

            fn ids(&self) -> Vec<TypeId>  {
                vec![
                    $(TypeId::of::<$t>(),)*
                ]
            }

            fn storage_locations(&self) -> Vec<StorageType> {
                vec![
                    $(StorageType::of::<$t>(),)*
                ]
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
