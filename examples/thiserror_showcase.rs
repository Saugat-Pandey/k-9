use kv_store::{KvStore, Key, OwnedValue, KvResult};

fn main() -> KvResult<()> {
    let mut store = KvStore::new();

    store.insert(Key::Text("age".into()), OwnedValue::Integer(20));

    let age: Option<OwnedValue> = store.get_owned(&Key::Text("age".into()))?;
    match age {
        Some(v) => println!("age found: {:?}", v),
        None => println!("age not set"),
    }

    let missing: Option<OwnedValue> =
        store.get_owned(&Key::Text("does_not_exist".into()))?;
    if missing.is_none() {
        println!("key 'does_not_exist' is not in the store");
    }

    Ok(())
}
