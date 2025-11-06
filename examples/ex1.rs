use k9_store::{KvStore, Key, OwnedValue};

fn main() {
    let mut kv = KvStore::new();
    kv.insert(Key::Text("language".into()), OwnedValue::Text("Rust".into()));
    assert!(matches!(
        kv.get_borrowed(&Key::Text("language".into())),
        Some(_)
    ));
}
