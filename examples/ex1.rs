use kv_store::{KvStore, Key, OwnedValue};

fn main() {
    let mut kv = KvStore::new();

    kv.insert(
        Key::Text("demo".into()),
        OwnedValue::Integer(123),
    );

    println!("Stored demo=123");
}
