use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Text(String),
    Number(i64),
}

#[derive(Debug, PartialEq)]
pub enum Value {
    Text(String),
    Number(i64),
    Bool(bool),
}

pub struct KvStore {
    store: HashMap<Key, Value>,
}

impl KvStore {

    pub fn new() -> Self {
        KvStore {
            store: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: Key, value: Value) {
        self.store.insert(key, value);
    }

    pub fn at(&self, key: &Key) -> Option<&Value> {
        self.store.get(key)
    }

    pub fn remove(&mut self, key: &Key) -> Option<Value> {
        self.store.remove(key)
    }

    pub fn erase(&mut self, key: &Key) -> Option<Value> {
        self.store.remove(key)
    }

    pub fn clear(&mut self) {
        self.store.clear();
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get_test() {
        let mut kv1 = KvStore::new();
        kv1.insert(Key::Text("language".into()), Value::Text("Rust".into()));
        assert_eq!(kv1.at(&Key::Text("language".into())), Some(&Value::Text("Rust".into())));
    }

      #[test]
    fn remove_test() {
        let mut kv2 = KvStore::new();
        kv2.insert(Key::Number(1), Value::Bool(true));

        assert_eq!(kv2.remove(&Key::Number(1)), Some(Value::Bool(true)));
        assert_eq!(kv2.at(&Key::Number(1)), None);
    }

    #[test]
    fn clear_all_test() {
        let mut kv3 = KvStore::new();
        kv3.insert(Key::Text("a".into()), Value::Text("A".into()));
        kv3.insert(Key::Text("b".into()), Value::Text("B".into()));
        kv3.clear();
        assert_eq!(kv3.at(&Key::Text("a".into())), None);
        assert_eq!(kv3.at(&Key::Text("b".into())), None);
    }
}