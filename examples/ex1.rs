use kv_store::{KvStore, Key, Value};

fn main() {
    let mut kv = KvStore::new();

    kv.insert(
        Key::Text("language".into()),
        Value::Text("Rust".into()),
    );
    assert_eq!(
        kv.at(&Key::Text("language".into())),
        Some(&Value::Text("Rust".into())),
    );

    kv.insert(
        Key::Number(1),
        Value::Bool(true),
    );
    assert_eq!(
        kv.at(&Key::Number(1)),
        Some(&Value::Bool(true)),
    );

    kv.insert(
        Key::Text("language".into()),
        Value::Text("C".into()),
    );
    assert_eq!(
        kv.at(&Key::Text("language".into())),
        Some(&Value::Text("C".into())),
    );

    let removed = kv.remove(&Key::Number(1));
    assert_eq!(removed, Some(Value::Bool(true)));
    assert_eq!(kv.at(&Key::Number(1)), None);

    kv.clear();
    assert_eq!(kv.at(&Key::Text("language".into())), None);

    println!("All example checks passed");
}
