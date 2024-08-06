use std::{collections::HashMap, hash::Hash};

pub struct Serializer<'a> {
    bytes: &'a mut Vec<u8>,
}

impl<'a> Serializer<'a> {
    pub fn new(buf: &'a mut Vec<u8>) -> Self {
        Self { bytes: buf }
    }

    pub fn push_bytes(&mut self, mut bytes: Vec<u8>) {
        self.bytes.append(&mut bytes);
    }

    pub fn push_collection<'c, T: Serialize>(
        &mut self,
        len: u32,
        collection: impl Iterator<Item = &'c T>,
    ) {
        for val in collection {
            val.serialize(self);
        }
        len.serialize(self);
    }

    pub fn push_keyed_collection<'c, K: Serialize, V: Serialize>(
        &mut self,
        len: u32,
        collection: impl Iterator<Item = (&'c K, &'c V)>,
    ) {
        for (key, val) in collection {
            val.serialize(self);
            key.serialize(self);
        }
        len.serialize(self);
    }
}

pub trait Serialize: 'static {
    fn serialize(&self, serializer: &mut Serializer<'_>);
}

impl<T: Serialize> Serialize for Box<T> {
    fn serialize(&self, serializer: &mut Serializer<'_>) {
        <T>::serialize(self, serializer);
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn serialize(&self, serializer: &mut Serializer<'_>) {
        let len = self.len();
        serializer.push_collection(len as u32, self.iter().rev());
    }
}

impl<K: Serialize + PartialEq + Eq + Hash, V: Serialize> Serialize for HashMap<K, V> {
    fn serialize(&self, serializer: &mut Serializer<'_>) {
        let len = self.len();
        serializer.push_keyed_collection(len as u32, self.iter());
    }
}

macro_rules! impl_serialize {
    ($t:ty) => {
        impl Serialize for $t {
            fn serialize(&self, serializer: &mut Serializer<'_>) {
                serializer.push_bytes(<$t>::to_le_bytes(*self).into());
            }
        }
    };
}

impl_serialize!(usize);
impl_serialize!(u128);
impl_serialize!(u64);
impl_serialize!(u32);
impl_serialize!(u16);
impl_serialize!(u8);

impl_serialize!(isize);
impl_serialize!(i128);
impl_serialize!(i64);
impl_serialize!(i32);
impl_serialize!(i16);
impl_serialize!(i8);
