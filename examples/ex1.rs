use k9_store::{KvStore, Key, Value};

fn main() {
    let mut kv = KvStore::new();
    kv.insert(Key::Text("language".into()), Value::Text("Rust".into()));

    assert_eq!(
        kv.at(&Key::Text("language".into())),
        Some(&Value::Text("Rust".into()))
    );

    println!("OK");
}
