use std::collections::HashMap;
use std::str;
use std::ptr;
use crc::{Crc, CRC_32_ISO_HDLC};

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
const CHECKSUM_BYTES: usize = 4;  // u32
const TAG_BYTES: usize = 1;       // u8
const HEADER_SIZE: usize = LEN_BYTES + CHECKSUM_BYTES + TAG_BYTES; // 13

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

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

#[repr(C, packed)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct RawHeader {
    length: u64,
    checksum: u32,
    tag: u8,
}

unsafe fn serialize_unsafe(header: &RawHeader, out: &mut Vec<u8>) {
    use std::mem;

    let header_size = mem::size_of::<RawHeader>();
    debug_assert_eq!(header_size, HEADER_SIZE);

    let old_len = out.len();
    out.reserve(header_size);

    unsafe {
        out.set_len(old_len + header_size);

        let dst = out.as_mut_ptr().add(old_len) as *mut u8;
        let src = header as *const RawHeader as *const u8;

        ptr::copy_nonoverlapping(src, dst, header_size);
    }
}

unsafe fn deserialize_unsafe(data: &[u8]) -> RawHeader {
    use std::mem;

    let header_size = mem::size_of::<RawHeader>();
    assert!(data.len() >= header_size, "Slice zu kurz für RawHeader");

    unsafe {
        let src = data.as_ptr() as *const RawHeader;
        std::ptr::read_unaligned(src)
    }
}

pub struct KvStore {
    data: Vec<u8>,
    pub(crate) index: HashMap<Key, usize>,
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
    let mut payload = Vec::new();
    let tag: TypeTag;

    match value {
        OwnedValue::Integer(x) => {
            tag = TypeTag::Integer;
            payload.extend_from_slice(&x.to_le_bytes());
        }
        OwnedValue::Bool(b) => {
            tag = TypeTag::Bool;
            payload.push(if *b { 1 } else { 0 });
        }
        OwnedValue::Text(s) => {
            tag = TypeTag::Text;
            let bytes = s.as_bytes();
            let len_u64 = bytes.len() as u64;
            payload.extend_from_slice(&len_u64.to_le_bytes());
            payload.extend_from_slice(bytes);
        }
        OwnedValue::Blob(v) => {
            tag = TypeTag::Blob;
            let len_u64 = v.len() as u64;
            payload.extend_from_slice(&len_u64.to_le_bytes());
            payload.extend_from_slice(v);
        }
    }

    // length: checksum + tag + payload
    let length: u64 = (CHECKSUM_BYTES + TAG_BYTES + payload.len()) as u64;
    let checksum = CRC32.checksum(&payload);

    let header = RawHeader {
        length,
        checksum,
        tag: tag as u8,
    };

    unsafe {
        serialize_unsafe(&header, out);
    }

    out.extend_from_slice(&payload);
}

fn deserialize_borrowed(data: &[u8]) -> BorrowedValue<'_> {
    assert!(data.len() >= HEADER_SIZE, "Header zu kurz");

    let header = unsafe { deserialize_unsafe(data) };

    let total_len = header.length as usize;
    assert!(
        data.len() >= LEN_BYTES + total_len,
        "Gesamteintrag abgeschnitten"
    );

    let payload_len = total_len - CHECKSUM_BYTES - TAG_BYTES;
    let payload_start = HEADER_SIZE;
    let payload_end = payload_start + payload_len;

    assert!(data.len() >= payload_end, "Payload abgeschnitten");

    let payload = &data[payload_start..payload_end];

    let stored_checksum = header.checksum;
    let tag_byte = header.tag;

    let computed = CRC32.checksum(payload);
    assert!(
        computed == stored_checksum,
        "Checksum mismatch (computed={}, stored={})",
        computed,
        stored_checksum
    );

    let tag = TypeTag::from_u8(tag_byte).expect("unbekannter Typ-Tag");

    match tag {
        TypeTag::Integer => {
            assert!(payload.len() >= 8, "Integer-Payload fehlt");
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&payload[..8]);
            BorrowedValue::Integer(i64::from_le_bytes(buf))
        }
        TypeTag::Bool => {
            assert!(payload.len() >= 1, "Bool-Payload fehlt");
            BorrowedValue::Bool(payload[0] != 0)
        }
        TypeTag::Text => {
            assert!(payload.len() >= 8, "Text-Längenfeld fehlt");
            let slen = u64::from_le_bytes(payload[0..8].try_into().unwrap()) as usize;
            assert!(payload.len() >= 8 + slen, "Text-Payload fehlt");
            let s = str::from_utf8(&payload[8..8 + slen]).expect("ungültiges UTF-8");
            BorrowedValue::Text(s)
        }
        TypeTag::Blob => {
            assert!(payload.len() >= 8, "Blob-Längenfeld fehlt");
            let blen = u64::from_le_bytes(payload[0..8].try_into().unwrap()) as usize;
            assert!(payload.len() >= 8 + blen, "Blob-Payload fehlt");
            BorrowedValue::Blob(&payload[8..8 + blen])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ktxt(s: &str) -> Key { Key::Text(s.to_string()) }
    fn kint(i: i64) -> Key { Key::Integer(i) }

    #[test]
    fn header_size_matches_const() {
        assert_eq!(HEADER_SIZE, std::mem::size_of::<RawHeader>());
    }

    #[test]
    fn raw_header_unsafe_roundtrip_single() {
        let h = RawHeader {
            length: 123,
            checksum: 0xDEADBEEF,
            tag: TypeTag::Text as u8,
        };

        let mut buf = Vec::new();
        unsafe {
            serialize_unsafe(&h, &mut buf);
        }

        assert_eq!(buf.len(), HEADER_SIZE);

        let h2 = unsafe { deserialize_unsafe(&buf) };
        assert_eq!(h, h2);
    }

    #[test]
    fn raw_header_unsafe_roundtrip_multiple() {
        let h1 = RawHeader {
            length: 10,
            checksum: 1,
            tag: TypeTag::Integer as u8,
        };
        let h2 = RawHeader {
            length: 20,
            checksum: 2,
            tag: TypeTag::Blob as u8,
        };

        let mut buf = Vec::new();
        unsafe {
            serialize_unsafe(&h1, &mut buf);
            serialize_unsafe(&h2, &mut buf);
        }

        assert_eq!(buf.len(), 2 * HEADER_SIZE);

        let first = unsafe { deserialize_unsafe(&buf[0..HEADER_SIZE]) };
        let second = unsafe { deserialize_unsafe(&buf[HEADER_SIZE..2 * HEADER_SIZE]) };

        assert_eq!(first, h1);
        assert_eq!(second, h2);
    }

    #[test]
    fn insert_and_get_text() {
        let mut kv = KvStore::new();
        kv.insert(ktxt("lang"), OwnedValue::Text("Rust".into()));
        assert_eq!(
            kv.get_borrowed(&ktxt("lang")),
            Some(BorrowedValue::Text("Rust"))
        );
    }

    #[test]
    fn insert_and_get_integer_bool_blob() {
        let mut kv = KvStore::new();
        kv.insert(ktxt("answer"), OwnedValue::Integer(42));
        kv.insert(ktxt("flag"), OwnedValue::Bool(true));
        kv.insert(ktxt("raw"), OwnedValue::Blob(vec![9,8,7]));

        assert_eq!(
            kv.get_borrowed(&ktxt("answer")),
            Some(BorrowedValue::Integer(42))
        );
        assert_eq!(
            kv.get_borrowed(&ktxt("flag")),
            Some(BorrowedValue::Bool(true))
        );
        match kv.get_borrowed(&ktxt("raw")).unwrap() {
            BorrowedValue::Blob(b) => assert_eq!(b, &[9,8,7]),
            _ => panic!("Blob erwartet"),
        }
    }

    #[test]
    fn integer_key_works() {
        let mut kv = KvStore::new();
        kv.insert(kint(5), OwnedValue::Text("five".into()));
        assert_eq!(
            kv.get_borrowed(&kint(5)),
            Some(BorrowedValue::Text("five"))
        );
    }

    #[test]
    fn overwrite_key_updates_value_and_offset() {
        let mut kv = KvStore::new();
        kv.insert(ktxt("k"), OwnedValue::Text("old".into()));
        let off1 = kv.index[&ktxt("k")];
        kv.insert(ktxt("k"), OwnedValue::Text("new".into()));
        let off2 = kv.index[&ktxt("k")];
        assert!(off2 > off1);
        assert_eq!(
            kv.get_borrowed(&ktxt("k")),
            Some(BorrowedValue::Text("new"))
        );
    }

    #[test]
    #[should_panic]
    fn checksum_detects_corruption() {
        let mut kv = KvStore::new();
        kv.insert(ktxt("x"), OwnedValue::Integer(123));

        let off = kv.index[&ktxt("x")];
        let corrupt_idx = off + HEADER_SIZE;
        kv.data[corrupt_idx] ^= 0xFF;

        let _ = kv.get_borrowed(&ktxt("x")); 
    }
}
