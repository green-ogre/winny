use std::{collections::HashMap, hash::Hash, marker::PhantomData};

pub struct Deserializer<'a> {
    bytes: &'a mut Vec<u8>,
}

impl<'a> Deserializer<'a> {
    pub fn new(buf: &'a mut Vec<u8>) -> Self {
        Self { bytes: buf }
    }

    pub fn pop_bytes(&mut self, n: usize) -> Vec<u8> {
        self.bytes.drain(self.bytes.len() - n..).collect::<Vec<_>>()
    }

    pub fn pop_collection<T: Deserialize>(&mut self, len: u32) -> impl Iterator<Item = T> {
        let mut collection = Vec::with_capacity(len as usize);
        for _ in 0..len {
            collection.push(T::deserialize(self).unwrap())
        }

        collection.into_iter()
    }

    pub fn pop_keyed_collection<K: Deserialize, V: Deserialize>(
        &mut self,
        len: u32,
    ) -> impl Iterator<Item = (K, V)> {
        let mut collection = Vec::with_capacity(len as usize);
        for _ in 0..len {
            collection.push((K::deserialize(self).unwrap(), V::deserialize(self).unwrap()))
        }

        collection.into_iter().rev()
    }
}

pub trait Deserialize: Sized {
    fn deserialize(deserializer: &mut Deserializer<'_>) -> Option<Self>;
}

impl Deserialize for bool {
    fn deserialize(deserializer: &mut Deserializer<'_>) -> Option<Self> {
        Some(*deserializer.pop_bytes(1).first().unwrap() == 1)
    }
}

impl<T: Deserialize> Deserialize for Box<T> {
    fn deserialize(deserializer: &mut Deserializer<'_>) -> Option<Self> {
        Some(Box::new(T::deserialize(deserializer).unwrap()))
    }
}

impl<T: Deserialize> Deserialize for Vec<T> {
    fn deserialize(deserializer: &mut Deserializer<'_>) -> Option<Self> {
        let len = u32::deserialize(deserializer).unwrap();
        Some(deserializer.pop_collection(len as u32).collect())
    }
}

impl<K: Deserialize + PartialEq + Eq + Hash, V: Deserialize> Deserialize for HashMap<K, V> {
    fn deserialize(deserializer: &mut Deserializer<'_>) -> Option<Self> {
        let len = u32::deserialize(deserializer).unwrap();
        let mut map = HashMap::default();

        for (k, v) in deserializer.pop_keyed_collection(len as u32) {
            map.insert(k, v);
        }

        Some(map)
    }
}

impl<T: Deserialize, const LEN: usize> Deserialize for [T; LEN] {
    fn deserialize(deserializer: &mut Deserializer<'_>) -> Option<Self> {
        let mut elems = Vec::new();
        for _ in 0..LEN {
            elems.push(T::deserialize(deserializer).unwrap());
        }

        Some(unsafe { elems.try_into().unwrap_unchecked() })
    }
}

impl<T: 'static> Deserialize for PhantomData<T> {
    fn deserialize(_deserializer: &mut Deserializer<'_>) -> Option<Self> {
        Some(PhantomData)
    }
}

macro_rules! impl_deserialize {
    ($t:ty) => {
        impl Deserialize for $t {
            fn deserialize(deserializer: &mut Deserializer<'_>) -> Option<Self> {
                let size = std::mem::size_of::<$t>();
                let val = deserializer.pop_bytes(size).try_into().unwrap();
                Some(<$t>::from_le_bytes(val))
            }
        }
    };
}

impl_deserialize!(usize);
impl_deserialize!(u128);
impl_deserialize!(u64);
impl_deserialize!(u32);
impl_deserialize!(u16);
impl_deserialize!(u8);

impl_deserialize!(isize);
impl_deserialize!(i128);
impl_deserialize!(i64);
impl_deserialize!(i32);
impl_deserialize!(i16);
impl_deserialize!(i8);

impl_deserialize!(f64);
impl_deserialize!(f32);
