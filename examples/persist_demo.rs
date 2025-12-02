//cargo run --example persist_demo
// length(8) + checksum(4) + tag(1) + string length(8) + payload

use kv_store::{KvStore, Key, OwnedValue};

fn ktxt(s: &str) -> Key {
    Key::Text(s.to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "lab7_store.bin";

    let mut store = KvStore::load_from_file(path)?;

    println!("Starte Demo mit Datei: {}", path);

    if let Some(val) = store.get_owned(&ktxt("name"))? {
        println!("Vorher: name = {:?}", val);
    } else {
        println!("Vorher: name nicht gesetzt");
    }

    store.insert(ktxt("name"), OwnedValue::Text("Richard".into()));
    store.insert(ktxt("answer"), OwnedValue::Integer(42));

    store.persist_to_file(path)?;

    println!("Neue Werte gespeichert.");
    println!("Datei '{}' bleibt erhalten.", path);

    Ok(())
}
