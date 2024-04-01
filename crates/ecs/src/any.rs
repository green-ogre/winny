use crate::ComponentStorageType;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct TypeId(u64);

#[derive(Debug, Clone, Copy)]
pub struct TypeName(&'static str);

pub const ENTITY: TypeId = TypeId(0);

impl TypeId {
    pub fn new(id: u64) -> TypeId {
        TypeId(id)
    }

    pub fn consume(self) -> u64 {
        self.0
    }

    pub fn of<T: TypeGetter>() -> TypeId {
        T::type_id()
    }
}

impl TypeName {
    pub fn new(name: &'static str) -> TypeName {
        TypeName(name)
    }

    pub fn of<T: TypeGetter>() -> TypeName {
        T::type_name()
    }

    pub fn as_string(&self) -> String {
        self.0.to_owned()
    }
}

pub trait TypeGetter: 'static {
    fn type_id() -> TypeId;
    fn type_name() -> TypeName;
}

pub trait Any: 'static {
    fn type_id(&self) -> TypeId;
    fn type_name(&self) -> TypeName;
}

impl<T: TypeGetter + 'static> Any for T {
    fn type_id(&self) -> TypeId {
        T::type_id()
    }

    fn type_name(&self) -> TypeName {
        T::type_name()
    }
}

impl dyn Any {
    pub fn downcast_ref<T: TypeGetter>(&self) -> Option<&T> {
        if self.type_id() != T::type_id() {
            return None;
        }
        unsafe { Some(&*(self as *const dyn Any as *const T)) }
    }
}
