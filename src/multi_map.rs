use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

pub struct MultiMap {
    map: HashMap<TypeId, Box<dyn Any>>,
}

pub trait Key {
    type Value;
}

impl MultiMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert<K, V>(&mut self, value: V)
    where
        K: Key<Value = V> + 'static,
        V: 'static,
    {
        let key = TypeId::of::<K>();
        let value: Box<dyn Any> = Box::new(value);

        self.map.insert(key, value);
    }

    pub fn get<K, V>(&self) -> Option<&V>
    where
        K: Key<Value = V> + 'static,
        V: 'static,
    {
        let key = TypeId::of::<K>();
        let value = self.map.get(&key)?.downcast_ref::<V>().unwrap();

        Some(value)
    }
}
