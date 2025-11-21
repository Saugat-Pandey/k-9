use std::collections::HashMap;
use std::str;
use crc::{Crc, CRC_32_ISO_HDLC};

#[cfg(test)]
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
#[cfg(test)]
use std::alloc::System;
#[cfg(test)]
#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

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


// SAFETY: This function copies a RawHeader struct directly into the Vec’s
// internal buffer using raw pointers. The caller ensures that:
// - 'header' is a valid RawHeader reference.
// - 'out' is a valid Vec<u8>.
// - We reserve space and then set_len, so the destination memory is valid
//   and writable before copying.
// - Source (stack struct) and destination (Vec buffer) do not overlap.
unsafe fn serialize_unsafe(header: &RawHeader, out: &mut Vec<u8>) {
    use std::mem;

    let header_size = mem::size_of::<RawHeader>();

    if header_size != HEADER_SIZE {
        panic!(
            "Internal error: RawHeader has size {header_size}, expected {HEADER_SIZE}"
        );
    }

    let old_len = out.len();
    out.reserve(header_size);

    unsafe {
        let base: *mut u8 = out.as_mut_ptr();
        let dst: *mut u8 = base.add(old_len);

        let src = header as *const RawHeader as *const u8;

        std::ptr::copy_nonoverlapping(src, dst, header_size);

        out.set_len(old_len + header_size);
    }
}

// SAFETY: This function reads a RawHeader from the beginning of the slice.
// The caller must ensure that 'data' has at least HEADER_SIZE bytes.
unsafe fn deserialize_unsafe(data: &[u8]) -> RawHeader {
    use std::mem;

    let header_size = mem::size_of::<RawHeader>();

    if data.len() < header_size {
        panic!("Slice zu kurz für RawHeader");
    }

    unsafe {
        let src = data.as_ptr() as *const RawHeader;
        std::ptr::read_unaligned(src)
    }
}

#[derive(Debug, PartialEq)]
pub struct BorrowedEntry<'a> {
    pub key: &'a Key,
    pub value: BorrowedValue<'a>,
}

pub struct KvStore {
    data: Vec<u8>,
    pub(crate) index: HashMap<Key, usize>,
}

pub struct StoreIter<'a> {
    buf: &'a [u8],
    pos: usize,
    index: &'a HashMap<Key, usize>,
}

fn parse_entry(data: &[u8]) -> Option<(BorrowedValue<'_>, usize)> {
    if data.len() < HEADER_SIZE {
        return None;
    }

    let header = unsafe { deserialize_unsafe(data) };

    let total_len = header.length as usize;

    let used = LEN_BYTES + total_len;

    if data.len() < used {
        return None;
    }

    let entry_slice = &data[..used];
    let val = deserialize_borrowed(entry_slice);

    Some((val, used))
}

impl<'a> Iterator for StoreIter<'a> {
    type Item = BorrowedEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.pos >= self.buf.len() {
                return None;
            }

            let slice = &self.buf[self.pos..];

            let parsed = parse_entry(slice);
            
            if parsed.is_none() {
                return None;
            }

            let (value, used) = parsed.unwrap();

            let current_off = self.pos;
            self.pos += used;

            let mut key_opt: Option<&'a Key> = None;

            for (k, off) in self.index.iter() {
                if *off == current_off {
                    key_opt = Some(k);
                    break;
                }
            }

            match key_opt {
                Some(kref) => {
                    let entry = BorrowedEntry {
                        key: kref,
                        value,
                    };
                    return Some(entry);
                }
                None => {
                    continue;
                }
            }
        }
    }
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

    pub fn iter(&self) -> StoreIter<'_> {
        StoreIter {
            buf: &self.data,
            pos: 0,
            index: &self.index,
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &Key> + '_ {
        self.iter().map(|entry| entry.key)
    }

    pub fn values(&self) -> impl Iterator<Item = BorrowedValue<'_>> + '_ {
        self.iter().map(|entry| entry.value)
    }
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
    if data.len() < HEADER_SIZE {
        panic!("Header too short");
    }

    let header = unsafe { deserialize_unsafe(data) };

    let total_len = header.length as usize;
    if data.len() < LEN_BYTES + total_len {
        panic!("Entry truncated (length field exceeds available data)");
    }

    let payload_len = total_len - CHECKSUM_BYTES - TAG_BYTES;
    let payload_start = HEADER_SIZE;
    let payload_end = payload_start + payload_len;

    if data.len() < payload_end {
        panic!("Payload truncated");
    }

    let payload = &data[payload_start..payload_end];

    let stored_checksum = header.checksum;
    let tag_byte = header.tag;

    let computed = CRC32.checksum(payload);
    if computed != stored_checksum {
        panic!(
            "Checksum mismatch (computed={}, stored={})",
            computed,
            stored_checksum
        );
    }

    let tag = match TypeTag::from_u8(tag_byte) {
        Some(t) => t,
        None => panic!("Unknown type tag"),
    };

    match tag {
        TypeTag::Integer => {
            if payload.len() < 8 {
                panic!("Missing integer payload");
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&payload[..8]);
            BorrowedValue::Integer(i64::from_le_bytes(buf))
        }
        TypeTag::Bool => {
            if payload.len() < 1 {
                panic!("Missing bool payload");
            }
            BorrowedValue::Bool(payload[0] != 0)
        }
        TypeTag::Text => {
            if payload.len() < 8 {
                panic!("Missing text length field");
            }
            let slen =
                u64::from_le_bytes(payload[0..8].try_into().unwrap()) as usize;

            if payload.len() < 8 + slen {
                panic!("Missing text payload");
            }
            let s = match str::from_utf8(&payload[8..8 + slen]) {
                Ok(v) => v,
                Err(_) => panic!("Invalid UTF-8 in text payload"),
            };
            BorrowedValue::Text(s)
        }
        TypeTag::Blob => {
            if payload.len() < 8 {
                panic!("Missing blob length field");
            }
            let blen =
                u64::from_le_bytes(payload[0..8].try_into().unwrap()) as usize;

            if payload.len() < 8 + blen {
                panic!("Missing blob payload");
            }
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
    #[should_panic]
    fn checksum_detects_corruption() {
        let mut kv = KvStore::new();
        kv.insert(ktxt("x"), OwnedValue::Integer(123));

        let off = kv.index[&ktxt("x")];
        let corrupt_idx = off + HEADER_SIZE;
        kv.data[corrupt_idx] ^= 0xFF;

        let _ = kv.get_borrowed(&ktxt("x"));
    }

    #[test]
    fn iteration_does_not_allocate_heap_memory() {
        let mut kv = KvStore::new();
        kv.insert(ktxt("a"), OwnedValue::Integer(1));
        kv.insert(ktxt("b"), OwnedValue::Integer(2));

        let reg = Region::new(GLOBAL);

        for v in kv.iter() {
            std::mem::drop(v);
        }

        let stats = reg.change();

        assert_eq!(stats.allocations, 0);
        assert_eq!(stats.bytes_allocated, 0);
    }
}
