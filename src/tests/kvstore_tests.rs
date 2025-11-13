use kv_store::{KvStore, Key, OwnedValue, BorrowedValue};

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
    kv.insert(ktxt("framework"), OwnedValue::Text("Tokio".into()));

    assert_eq!(
        kv.get_borrowed(&ktxt("lang")),
        Some(BorrowedValue::Text("Rust"))
    );
    assert_eq!(
        kv.get_borrowed(&ktxt("framework")),
        Some(BorrowedValue::Text("Tokio"))
    );

    assert_eq!(
        kv.get_owned(&ktxt("lang")),
        Some(OwnedValue::Text("Rust".into()))
    );
}

#[test]
fn insert_and_get_integer_bool_blob() {
    let mut kv = KvStore::new();

    kv.insert(ktxt("answer"), OwnedValue::Integer(42));
    kv.insert(ktxt("flag"), OwnedValue::Bool(true));
    kv.insert(ktxt("raw"), OwnedValue::Blob(vec![9, 8, 7]));

    assert_eq!(
        kv.get_borrowed(&ktxt("answer")),
        Some(BorrowedValue::Integer(42))
    );
    assert_eq!(
        kv.get_borrowed(&ktxt("flag")),
        Some(BorrowedValue::Bool(true))
    );

    match kv.get_borrowed(&ktxt("raw")).unwrap() {
        BorrowedValue::Blob(b) => assert_eq!(b, &[9, 8, 7]),
        _ => panic!("expected Blob"),
    }

    assert_eq!(
        kv.get_owned(&ktxt("answer")),
        Some(OwnedValue::Integer(42))
    );
}

#[test]
fn integer_key_works_and_is_distinct_from_text_key() {
    let mut kv = KvStore::new();

    kv.insert(kint(5), OwnedValue::Text("five (int)".into()));
    kv.insert(ktxt("5"), OwnedValue::Text("five (string)".into()));

    assert_eq!(
        kv.get_borrowed(&kint(5)),
        Some(BorrowedValue::Text("five (int)"))
    );
    assert_eq!(
        kv.get_borrowed(&ktxt("5")),
        Some(BorrowedValue::Text("five (string)"))
    );
}

#[test]
fn overwrite_key_updates_value() {
    let mut kv = KvStore::new();

    kv.insert(ktxt("k"), OwnedValue::Text("old".into()));
    kv.insert(ktxt("k"), OwnedValue::Text("new".into()));

    assert_eq!(
        kv.get_borrowed(&ktxt("k")),
        Some(BorrowedValue::Text("new"))
    );
    assert_eq!(
        kv.get_owned(&ktxt("k")),
        Some(OwnedValue::Text("new".into()))
    );
}

#[test]
fn missing_key_returns_none() {
    let mut kv = KvStore::new();

    kv.insert(ktxt("exists"), OwnedValue::Integer(1));

    assert_eq!(kv.get_borrowed(&ktxt("does_not_exist")), None);
    assert_eq!(kv.get_owned(&ktxt("does_not_exist")), None);
}

#[test]
fn multiple_entries_do_not_interfere() {
    let mut kv = KvStore::new();

    kv.insert(ktxt("a"), OwnedValue::Integer(1));
    kv.insert(ktxt("b"), OwnedValue::Bool(false));
    kv.insert(ktxt("c"), OwnedValue::Text("see".into()));

    assert_eq!(
        kv.get_borrowed(&ktxt("a")),
        Some(BorrowedValue::Integer(1))
    );
    assert_eq!(
        kv.get_borrowed(&ktxt("b")),
        Some(BorrowedValue::Bool(false))
    );
    assert_eq!(
        kv.get_borrowed(&ktxt("c")),
        Some(BorrowedValue::Text("see"))
    );
}
