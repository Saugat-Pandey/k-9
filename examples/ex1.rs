use kv_store::{KvStore, Key, OwnedValue, BorrowedValue};

fn ktxt(s: &str) -> Key { Key::Text(s.to_string()) }
fn kint(i: i64) -> Key { Key::Integer(i) }

fn main() {
    let mut kv = KvStore::new();



    assert_eq!(kv.get_borrowed(&ktxt("missing")), None);
    assert_eq!(kv.get_owned(&ktxt("missing")), None);
    assert_eq!(kv.storage_len(), 0);



    kv.insert(ktxt("language"), OwnedValue::Text("Rust".into()));
    assert_eq!(
        kv.get_borrowed(&ktxt("language")),
        Some(BorrowedValue::Text("Rust")),
    );
    assert_eq!(
        kv.get_owned(&ktxt("language")),
        Some(OwnedValue::Text("Rust".into())),
    );
    let len_after_text = kv.storage_len();



    kv.insert(ktxt("answer"), OwnedValue::Integer(42));
    assert!(kv.storage_len() > len_after_text);
    assert_eq!(
        kv.get_borrowed(&ktxt("answer")),
        Some(BorrowedValue::Integer(42)),
    );
    assert_eq!(
        kv.get_owned(&ktxt("answer")),
        Some(OwnedValue::Integer(42)),
    );



    kv.insert(ktxt("flag"), OwnedValue::Bool(true));
    assert_eq!(
        kv.get_borrowed(&ktxt("flag")),
        Some(BorrowedValue::Bool(true)),
    );
    assert_eq!(
        kv.get_owned(&ktxt("flag")),
        Some(OwnedValue::Bool(true)),
    );



    kv.insert(ktxt("blob"), OwnedValue::Blob(vec![1, 2, 3, 4]));
    match kv.get_borrowed(&ktxt("blob")).unwrap() {
        BorrowedValue::Blob(slice) => assert_eq!(slice, &[1, 2, 3, 4]),
        _ => panic!("Blob erwartet"),
    }
    match kv.get_owned(&ktxt("blob")).unwrap() {
        OwnedValue::Blob(vec) => assert_eq!(vec, vec![1, 2, 3, 4]),
        _ => panic!("Owned Blob erwartet"),
    }



    kv.insert(kint(5), OwnedValue::Text("five".into()));
    kv.insert(kint(-1), OwnedValue::Bool(false));
    assert_eq!(
        kv.get_borrowed(&kint(5)),
        Some(BorrowedValue::Text("five")),
    );
    assert_eq!(
        kv.get_borrowed(&kint(-1)),
        Some(BorrowedValue::Bool(false)),
    );



    let off_before = kv.storage_len();
    kv.insert(ktxt("language"), OwnedValue::Text("C".into()));
    let off_after = kv.storage_len();
    assert!(off_after > off_before);
    assert_eq!(
        kv.get_borrowed(&ktxt("language")),
        Some(BorrowedValue::Text("C")),
    );
    assert_eq!(
        kv.get_owned(&ktxt("language")),
        Some(OwnedValue::Text("C".into())),
    );

    assert_eq!(kv.get_borrowed(&ktxt("does-not-exist")), None);

    println!("All example checks passed for Lab3");
}
