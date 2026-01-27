use kv_store::{KvStore, Key, OwnedValue, BorrowedValue, BorrowedEntry};

fn ktxt(s: &str) -> Key {
    Key::Text(s.to_string())
}

fn kint(i: i64) -> Key {
    Key::Integer(i)
}

#[test]
fn insert_and_get_text() {
    let mut kv = KvStore::new();
    kv.insert(ktxt("lang"), OwnedValue::Text("Rust".into()));

    // get_borrowed -> Result<Option<BorrowedValue>>
    assert_eq!(
        kv.get_borrowed(&ktxt("lang")).unwrap(),
        Some(BorrowedValue::Text("Rust"))
    );
}

#[test]
fn insert_and_get_integer_bool_blob() {
    let mut kv = KvStore::new();
    kv.insert(ktxt("answer"), OwnedValue::Integer(42));
    kv.insert(ktxt("flag"), OwnedValue::Bool(true));
    kv.insert(ktxt("raw"), OwnedValue::Blob(vec![9, 8, 7]));

    assert_eq!(
        kv.get_borrowed(&ktxt("answer")).unwrap(),
        Some(BorrowedValue::Integer(42))
    );
    assert_eq!(
        kv.get_borrowed(&ktxt("flag")).unwrap(),
        Some(BorrowedValue::Bool(true))
    );

    // hier Result UND Option auspacken, ganz stumpf:
    match kv.get_borrowed(&ktxt("raw")).unwrap().unwrap() {
        BorrowedValue::Blob(b) => assert_eq!(b, &[9, 8, 7]),
        _ => panic!("expected Blob"),
    }

    assert_eq!(
        kv.get_owned(&ktxt("answer")).unwrap(),
        Some(OwnedValue::Integer(42))
    );
}

#[test]
fn overwrite_key_updates_value() {
    let mut kv = KvStore::new();
    kv.insert(ktxt("k"), OwnedValue::Text("old".into()));
    kv.insert(ktxt("k"), OwnedValue::Text("new".into()));

    assert_eq!(
        kv.get_borrowed(&ktxt("k")).unwrap(),
        Some(BorrowedValue::Text("new"))
    );
}

#[test]
fn iter_returns_entries_in_storage_order() {
    let mut kv = KvStore::new();

    kv.insert(ktxt("a"), OwnedValue::Integer(10));
    kv.insert(ktxt("b"), OwnedValue::Bool(true));
    kv.insert(ktxt("c"), OwnedValue::Text("hello".into()));

    let items: Vec<(Key, BorrowedValue)> =
        kv.iter().map(|e| (e.key.clone(), e.value)).collect();

    assert_eq!(items.len(), 3);

    assert_eq!(items[0].0, ktxt("a"));
    assert_eq!(items[0].1, BorrowedValue::Integer(10));

    assert_eq!(items[1].0, ktxt("b"));
    assert_eq!(items[1].1, BorrowedValue::Bool(true));

    assert_eq!(items[2].0, ktxt("c"));
    assert_eq!(items[2].1, BorrowedValue::Text("hello"));
}

#[test]
fn iter_stops_correctly() {
    let mut kv = KvStore::new();
    kv.insert(ktxt("x"), OwnedValue::Integer(1));

    let mut it = kv.iter();

    // nicht BorrowedEntry direkt vergleichen (Lifetimes), sondern Felder prüfen
    let first: BorrowedEntry<'_> = it.next().expect("expected one element");
    assert_eq!(first.key, &ktxt("x"));
    assert_eq!(first.value, BorrowedValue::Integer(1));

    assert_eq!(it.next(), None);
    assert_eq!(it.next(), None);
}

#[test]
fn keys_are_returned_in_storage_order() {
    let mut kv = KvStore::new();
    kv.insert(ktxt("first"), OwnedValue::Integer(1));
    kv.insert(ktxt("second"), OwnedValue::Integer(2));
    kv.insert(ktxt("third"), OwnedValue::Integer(3));

    let keys: Vec<&Key> = kv.keys().collect();

    assert_eq!(keys.len(), 3);
    assert_eq!(keys[0], &ktxt("first"));
    assert_eq!(keys[1], &ktxt("second"));
    assert_eq!(keys[2], &ktxt("third"));
}

#[test]
fn values_returned_in_storage_order() {
    let mut kv = KvStore::new();
    kv.insert(kint(5), OwnedValue::Text("five".into()));
    kv.insert(kint(6), OwnedValue::Text("six".into()));

    let vals: Vec<_> = kv.values().collect();

    assert_eq!(vals.len(), 2);
    assert_eq!(vals[0], BorrowedValue::Text("five"));
    assert_eq!(vals[1], BorrowedValue::Text("six"));
}

#[test]
fn iter_works_with_all_types() {
    let mut kv = KvStore::new();

    kv.insert(kint(1), OwnedValue::Integer(42));
    kv.insert(kint(2), OwnedValue::Bool(false));
    kv.insert(kint(3), OwnedValue::Blob(vec![1, 2, 3]));

    let vals: Vec<_> = kv.iter().map(|e| e.value).collect();

    assert_eq!(vals.len(), 3);
    assert_eq!(vals[0], BorrowedValue::Integer(42));
    assert_eq!(vals[1], BorrowedValue::Bool(false));
    match vals[2] {
        BorrowedValue::Blob(b) => assert_eq!(b, &[1, 2, 3]),
        _ => panic!("expected blob"),
    }
}

#[test]
fn persist_and_load_roundtrip() {
    let path = "test_store_roundtrip.bin";

    {
        let mut kv = KvStore::new();
        kv.insert(ktxt("a"), OwnedValue::Integer(1));
        kv.insert(ktxt("b"), OwnedValue::Text("hello".into()));
        kv.persist_to_file(path).unwrap();
    }

    let kv2 = KvStore::load_from_file(path).unwrap();

    assert_eq!(
        kv2.get_owned(&ktxt("a")).unwrap(),
        Some(OwnedValue::Integer(1))
    );
    assert_eq!(
        kv2.get_owned(&ktxt("b")).unwrap(),
        Some(OwnedValue::Text("hello".into()))
    );

    let _ = std::fs::remove_file(path);
}

#[test]
fn load_missing_file_returns_empty_store() {
    let path = "definitely_not_existing_12345.bin";
    let _ = std::fs::remove_file(path);

    let kv = KvStore::load_from_file(path).unwrap();
    let count = kv.keys().count();

    assert_eq!(count, 0);
}

#[test]
fn load_corrupted_file_returns_error() {
    use std::fs::File;
    use std::io::{Read, Write};

    let path = "corrupted_store.bin";

    {
        let mut kv = KvStore::new();
        kv.insert(ktxt("k"), OwnedValue::Integer(123));
        kv.persist_to_file(path).unwrap();
    }

    // Datei einlesen, ein Byte verändern, zurückschreiben
    {
        let mut data = Vec::new();
        {
            let mut f = File::open(path).unwrap();
            f.read_to_end(&mut data).unwrap();
        }

        if !data.is_empty() {
            let mid = data.len() / 2;
            data[mid] ^= 0xFF;
        }

        let mut f = File::create(path).unwrap();
        f.write_all(&data).unwrap();
    }

    let res = KvStore::load_from_file(path);

    assert!(res.is_err());

    let _ = std::fs::remove_file(path);
}
