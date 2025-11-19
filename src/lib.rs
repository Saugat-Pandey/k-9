// Lab 3 does not require checksum handling. We included the checksum field
// anyway to match the final header format used in Lab 4. For now it acts only
// as a "placeholder"



use std::collections::HashMap;
use std::str;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Text(String),
    Integer(i64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum OwnedValue {
    Integer(i64),
    Bool(bool),
    Text(String),
    Blob(Vec<u8>),
}

#[derive(Debug, PartialEq)]
pub enum BorrowedValue<'a> {
    Integer(i64),
    Bool(bool),
    Text(&'a str),
    Blob(&'a [u8]),
}

impl<'a> BorrowedValue<'a> {
    pub fn to_owned(&self) -> OwnedValue {
        match self {
            BorrowedValue::Integer(x) => OwnedValue::Integer(*x),
            BorrowedValue::Bool(b) => OwnedValue::Bool(*b),
            BorrowedValue::Text(s) => OwnedValue::Text(s.to_string()),
            BorrowedValue::Blob(bytes) => OwnedValue::Blob(bytes.to_vec()),
        }
    }
}

const LEN_BYTES: usize = 8;       // u64
const CHECKSUM_BYTES: usize = 4;  // u32 (noch ungenutzt)
const TAG_BYTES: usize = 1;       // u8
const HEADER_SIZE: usize = LEN_BYTES + CHECKSUM_BYTES + TAG_BYTES; // 13

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
enum TypeTag {
    Integer = 0,
    Text = 1,
    Bool = 2,
    Blob = 3,
}

impl TypeTag {
    fn from_u8(b: u8) -> Option<TypeTag> {
        match b {
            0 => Some(TypeTag::Integer),
            1 => Some(TypeTag::Text),
            2 => Some(TypeTag::Bool),
            3 => Some(TypeTag::Blob),
            _ => None,
        }
    }
}


pub struct KvStore {
    data: Vec<u8>,                 
    index: HashMap<Key, usize>,
}

impl KvStore {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            index: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: Key, value: OwnedValue) {
        let offset = self.data.len();
        serialize_value(&value, &mut self.data);
        self.index.insert(key, offset);
    }

    pub fn get_borrowed(&self, key: &Key) -> Option<BorrowedValue<'_>> {
        match self.index.get(key) {
            Some(&off) => Some(deserialize_borrowed(&self.data[off..])),
            None => None,
        }
    }


    pub fn get_owned(&self, key: &Key) -> Option<OwnedValue> {
        match self.get_borrowed(key) {
            Some(borrowed) => Some(borrowed.to_owned()),
            None => None,
        }
    }


    #[allow(dead_code)]
    pub fn storage_len(&self) -> usize { self.data.len() }
}

fn serialize_value(value: &OwnedValue, out: &mut Vec<u8>) {
    match value {
        OwnedValue::Integer(x) => {
            let length: u64 = (CHECKSUM_BYTES + TAG_BYTES + 8) as u64;
            out.extend_from_slice(&length.to_le_bytes());    // u64
            out.extend_from_slice(&0u32.to_le_bytes());      // checksum = 0 (später Lab 4)
            out.push(TypeTag::Integer as u8);                // tag
            out.extend_from_slice(&x.to_le_bytes());         // payload
        }
        OwnedValue::Bool(b) => {
            let length: u64 = (CHECKSUM_BYTES + TAG_BYTES + 1) as u64;
            out.extend_from_slice(&length.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.push(TypeTag::Bool as u8);
            out.push(if *b { 1 } else { 0 });
        }
        OwnedValue::Text(s) => {
            let bytes = s.as_bytes();
            let len_u64 = bytes.len() as u64;
            let length: u64 = (CHECKSUM_BYTES + TAG_BYTES) as u64 + 8 + len_u64;
            out.extend_from_slice(&length.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.push(TypeTag::Text as u8);
            out.extend_from_slice(&len_u64.to_le_bytes());
            out.extend_from_slice(bytes);
        }
        OwnedValue::Blob(v) => {
            let len_u64 = v.len() as u64;
            let length: u64 = (CHECKSUM_BYTES + TAG_BYTES) as u64 + 8 + len_u64;
            out.extend_from_slice(&length.to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            out.push(TypeTag::Blob as u8);
            out.extend_from_slice(&len_u64.to_le_bytes());
            out.extend_from_slice(v);
        }
    }
}

fn deserialize_borrowed(data: &[u8]) -> BorrowedValue<'_> {
    assert!(data.len() >= HEADER_SIZE, "Header zu kurz");

    let length = u64::from_le_bytes(data[0..8].try_into().unwrap()) as usize;
    assert!(data.len() >= LEN_BYTES + length, "Gesamteintrag abgeschnitten");

    let _checksum = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let tag = data[12];

    match TypeTag::from_u8(tag).expect("unbekannter Typ-Tag") {
        TypeTag::Integer => {
            assert!(data.len() >= HEADER_SIZE + 8, "Integer-Payload fehlt");
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&data[HEADER_SIZE..HEADER_SIZE + 8]);
            BorrowedValue::Integer(i64::from_le_bytes(buf))
        }
        TypeTag::Bool => {
            assert!(data.len() >= HEADER_SIZE + 1, "Bool-Payload fehlt");
            BorrowedValue::Bool(data[HEADER_SIZE] != 0)
        }
        TypeTag::Text => {
            assert!(data.len() >= HEADER_SIZE + 8, "Text-Längenfeld fehlt");
            let slen = u64::from_le_bytes(data[HEADER_SIZE..HEADER_SIZE + 8].try_into().unwrap()) as usize;
            assert!(data.len() >= HEADER_SIZE + 8 + slen, "Text-Payload fehlt");
            let start = HEADER_SIZE + 8;
            let end = start + slen;
            let s = str::from_utf8(&data[start..end]).expect("ungültiges UTF-8");
            BorrowedValue::Text(s)
        }
        TypeTag::Blob => {
            assert!(data.len() >= HEADER_SIZE + 8, "Blob-Längenfeld fehlt");
            let blen = u64::from_le_bytes(data[HEADER_SIZE..HEADER_SIZE + 8].try_into().unwrap()) as usize;
            assert!(data.len() >= HEADER_SIZE + 8 + blen, "Blob-Payload fehlt");
            let start = HEADER_SIZE + 8;
            let end = start + blen;
            BorrowedValue::Blob(&data[start..end])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ktxt(s: &str) -> Key { Key::Text(s.to_string()) }
    fn kint(i: i64) -> Key { Key::Integer(i) }

    #[test]
    fn insert_and_get_text() {
        let mut kv = KvStore::new();
        kv.insert(ktxt("lang"), OwnedValue::Text("Rust".into()));
        assert_eq!(kv.get_borrowed(&ktxt("lang")), Some(BorrowedValue::Text("Rust")));
    }

    #[test]
    fn insert_and_get_integer_bool_blob() {
        let mut kv = KvStore::new();
        kv.insert(ktxt("answer"), OwnedValue::Integer(42));
        kv.insert(ktxt("flag"), OwnedValue::Bool(true));
        kv.insert(ktxt("raw"), OwnedValue::Blob(vec![9,8,7]));

        assert_eq!(kv.get_borrowed(&ktxt("answer")), Some(BorrowedValue::Integer(42)));
        assert_eq!(kv.get_borrowed(&ktxt("flag")), Some(BorrowedValue::Bool(true)));
        match kv.get_borrowed(&ktxt("raw")).unwrap() {
            BorrowedValue::Blob(b) => assert_eq!(b, &[9,8,7]),
            _ => panic!("Blob erwartet"),
        }
    }

    #[test]
    fn integer_key_works() {
        let mut kv = KvStore::new();
        kv.insert(kint(5), OwnedValue::Text("five".into()));
        assert_eq!(kv.get_borrowed(&kint(5)), Some(BorrowedValue::Text("five")));
    }

    #[test]
    fn overwrite_key_updates_value_and_offset() {
        let mut kv = KvStore::new();
        kv.insert(ktxt("k"), OwnedValue::Text("old".into()));
        let off1 = kv.index[&ktxt("k")];
        kv.insert(ktxt("k"), OwnedValue::Text("new".into()));
        let off2 = kv.index[&ktxt("k")];
        assert!(off2 > off1);
        assert_eq!(kv.get_borrowed(&ktxt("k")), Some(BorrowedValue::Text("new")));
    }
}