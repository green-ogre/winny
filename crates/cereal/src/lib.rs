pub mod deserialize;
pub mod serialize;

pub use crate::{deserialize::*, serialize::*};
extern crate self as cereal;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Deserializer, Serializer};
    use cereal_macro::{Deserialize, Serialize};
    use std::{
        any::{Any, TypeId},
        collections::HashMap,
        fmt::Debug,
    };

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct SomeData {
        x: u64,
        y: i8,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct SomeBox {
        x: u64,
        b: Box<usize>,
    }

    trait Reflect: Any + Debug {
        fn insert(&self);
    }

    impl Reflect for SomeData {
        fn insert(&self) {
            println!("{self:?}");
        }
    }

    impl Reflect for SomeBox {
        fn insert(&self) {
            println!("{self:?}");
        }
    }

    fn serialize_deserialize_struct<T: Serialize + Deserialize + Debug + PartialEq>(val: T) {
        let mut buf = Vec::new();
        let mut s = Serializer::new(&mut buf);
        val.serialize(&mut s);
        // println!("{val:?}");

        let mut d = Deserializer::new(&mut buf);
        let loaded_val = T::deserialize(&mut d);
        // println!("{loaded_val:?}");

        assert_eq!(val, loaded_val);
    }

    #[test]
    fn reflection() {
        let mut buf = Vec::new();
        let mut s = Serializer::new(&mut buf);
        let val = SomeData { x: 10, y: 1 };
        val.serialize(&mut s);
        let deserialize: fn(&mut Deserializer) -> Box<dyn Reflect> =
            |d| Box::new(SomeData::deserialize(d));

        // let mut id_to_de =
        //     HashMap::<TypeId, Box<dyn Fn(&mut Deserializer) -> Box<dyn Reflect>>>::new();
        // id_to_de.insert(TypeId::of::<SomeData>(), Box::new(deserialize));

        let mut d = Deserializer::new(&mut buf);
        let loaded_val = deserialize(&mut d);
        println!("{val:?}");
        loaded_val.insert();
    }

    #[test]
    fn simple() {
        let val = SomeData { x: 420, y: 69 };
        let val_2 = SomeData { x: 111, y: 33 };
        let mut buf = Vec::new();
        let mut s = Serializer::new(&mut buf);
        val.serialize(&mut s);
        val_2.serialize(&mut s);
        // println!("{val:?} {val_2:?}");

        let mut d = Deserializer::new(&mut buf);
        let loaded_val_2 = SomeData::deserialize(&mut d);
        let loaded_val = SomeData::deserialize(&mut d);
        // println!("{loaded_val:?} {loaded_val_2:?}");

        assert_eq!(val, loaded_val);
        assert_eq!(val_2, loaded_val_2);
    }

    #[test]
    fn box_test() {
        let val = SomeBox {
            x: 32,
            b: Box::new(12),
        };
        serialize_deserialize_struct(val);
    }

    #[test]
    fn vec() {
        let val = vec![
            SomeBox {
                x: 2,
                b: Box::new(111),
            },
            SomeBox {
                x: 32,
                b: Box::new(12),
            },
        ];

        serialize_deserialize_struct(val);
    }

    #[test]
    fn hash_map() {
        let mut val = HashMap::default();
        val.insert(
            100,
            SomeBox {
                x: 2,
                b: Box::new(111),
            },
        );
        val.insert(
            200,
            SomeBox {
                x: 32,
                b: Box::new(12),
            },
        );

        serialize_deserialize_struct(val);
    }

    #[test]
    fn all() {
        let mut hash = HashMap::<usize, SomeBox>::default();
        hash.insert(
            100,
            SomeBox {
                x: 2,
                b: Box::new(111),
            },
        );
        hash.insert(
            200,
            SomeBox {
                x: 32,
                b: Box::new(12),
            },
        );

        let vec = vec![
            SomeBox {
                x: 2,
                b: Box::new(111),
            },
            SomeBox {
                x: 32,
                b: Box::new(12),
            },
        ];

        let val = SomeData { x: 111, y: 33 };

        let mut buf = Vec::new();
        let mut s = Serializer::new(&mut buf);
        hash.serialize(&mut s);
        vec.serialize(&mut s);
        val.serialize(&mut s);

        let mut d = Deserializer::new(&mut buf);
        let loaded_val = SomeData::deserialize(&mut d);
        let loaded_vec = Vec::<SomeBox>::deserialize(&mut d);
        let loaded_hash = HashMap::<usize, SomeBox>::deserialize(&mut d);

        assert_eq!(val, loaded_val);
        assert_eq!(vec, loaded_vec);
        assert_eq!(hash, loaded_hash);
    }
}
