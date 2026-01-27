use std::collections::HashMap;
use std::str;

use crc::{Crc, CRC_32_ISO_HDLC};
use thiserror::Error;

#[cfg(test)]
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
#[cfg(test)]
use std::alloc::System;
#[cfg(test)]
#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[derive(Debug, Error)]
pub enum KvError {
    #[error("storage data is corrupted")]
    Corrupted(#[from] DecodeError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid key type in file")]
    InvalidKeyType,

    #[error("unexpected end of file while reading key/value pair")]
    UnexpectedEof,
}

#[derive(Debug, Error)]
enum DecodeError {
    #[error("slice too short for RawHeader")]
    SliceTooShortForHeader,
    #[error("entry truncated (length field exceeds available data)")]
    EntryTruncated,
    #[error("payload truncated")]
    PayloadTruncated,
    #[error("checksum mismatch (computed={computed} stored={stored})")]
    ChecksumMismatch { computed: u32, stored: u32 },
    #[error("unknown type tag {0}")]
    UnknownTypeTag(u8),
    #[error("missing integer payload")]
    MissingIntegerPayload,
    #[error("missing bool payload")]
    MissingBoolPayload,
    #[error("missing text length field")]
    MissingTextLength,
    #[error("missing text payload")]
    MissingTextPayload,
    #[error("invalid UTF-8 in text payload")]
    InvalidUtf8,
    #[error("missing blob length field")]
    MissingBlobLength,
    #[error("missing blob payload")]
    MissingBlobPayload,
}

pub type KvResult<T> = Result<T, KvError>;

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

// SAFETY: directly copies the RawHeader into the Vec’s buffer. The caller guarantees that
// the destination has sufficient space and the pointers do not overlap.
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

    let base: *mut u8 = out.as_mut_ptr();
    let dst: *mut u8 = base.add(old_len);
    let src = header as *const RawHeader as *const u8;

    std::ptr::copy_nonoverlapping(src, dst, header_size);

    out.set_len(old_len + header_size);
}

fn deserialize_header(data: &[u8]) -> Result<RawHeader, DecodeError> {
    use std::mem;

    let header_size = mem::size_of::<RawHeader>();

    if data.len() < header_size {
        return Err(DecodeError::SliceTooShortForHeader);
    }

    let src = data.as_ptr() as *const RawHeader;
    let header = unsafe { std::ptr::read_unaligned(src) };
    Ok(header)
}

#[derive(Debug, PartialEq)]
pub struct BorrowedEntry<'a> {
    pub key: &'a Key,
    pub value: BorrowedValue<'a>,
}

pub struct KvStore {
    data: Vec<u8>,
    index: HashMap<Key, usize>,
}

pub struct StoreIter<'a> {
    buf: &'a [u8],
    pos: usize,
    index: &'a HashMap<Key, usize>,
}

fn parse_entry(data: &[u8]) -> Result<Option<(BorrowedValue<'_>, usize)>, DecodeError> {
    if data.len() < HEADER_SIZE {
        return Ok(None);
    }

    let header = deserialize_header(data)?;
    let total_len = header.length as usize;

    let used = LEN_BYTES + total_len;

    if data.len() < used {
        return Ok(None);
    }

    let entry_slice = &data[..used];
    let val = deserialize_borrowed(entry_slice)?;

    Ok(Some((val, used)))
}

impl<'a> Iterator for StoreIter<'a> {
    type Item = BorrowedEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.pos >= self.buf.len() {
                return None;
            }

            let slice = &self.buf[self.pos..];

            let parsed = match parse_entry(slice) {
                Ok(v) => v,
                Err(_) => {
                    return None;
                }
            };

            let (value, used) = match parsed {
                Some(pair) => pair,
                None => return None,
            };

            let current_off = self.pos;
            self.pos += used;

            let mut key_opt: Option<&'a Key> = None;
            for (k, off) in self.index.iter() {
                if *off == current_off {
                    key_opt = Some(k);
                    break;
                }
            }

            if let Some(kref) = key_opt {
                let entry = BorrowedEntry {
                    key: kref,
                    value,
                };
                return Some(entry);
            } else {
                continue;
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

    pub fn get_borrowed(&self, key: &Key) -> KvResult<Option<BorrowedValue<'_>>> {
        match self.index.get(key) {
            Some(&off) => {
                let value =
                    deserialize_borrowed(&self.data[off..]).map_err(KvError::from)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    pub fn get_owned(&self, key: &Key) -> KvResult<Option<OwnedValue>> {
        match self.get_borrowed(key)? {
            Some(borrowed) => Ok(Some(borrowed.to_owned())),
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
    pub fn storage_len(&self) -> usize {
        self.data.len()
    }

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

    pub fn persist_to_file(&self, path: &str) -> KvResult<()> {
        use std::fs::File;
        use std::io::{BufWriter, Write};

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        for entry in self.iter() {
            let key_value = match entry.key {
                Key::Text(s) => OwnedValue::Text(s.clone()),
                Key::Integer(i) => OwnedValue::Integer(*i),
            };

            let value_owned = entry.value.to_owned();

            let mut buf = Vec::new();
            serialize_value(&key_value, &mut buf);
            serialize_value(&value_owned, &mut buf);

            writer.write_all(&buf)?;
        }

        writer.flush()?;
        Ok(())
    }


    pub fn load_from_file(path: &str) -> KvResult<KvStore> {
        use std::fs;
        use std::io::ErrorKind;

        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    return Ok(KvStore::new());
                } else {
                    return Err(KvError::Io(e));
                }
            }
        };

        let mut store = KvStore::new();
        let mut pos: usize = 0;

        while pos < bytes.len() {
            let slice_key = &bytes[pos..];

            let key_parsed = match parse_entry(slice_key) {
                Ok(v) => v,
                Err(e) => {
                    return Err(KvError::Corrupted(e));
                }
            };

            let (key_val, used_key) = match key_parsed {
                Some(pair) => pair,
                None => {
                    return Err(KvError::UnexpectedEof);
                }
            };

            pos += used_key;

            if pos >= bytes.len() {
                return Err(KvError::UnexpectedEof);
            }

            let slice_val = &bytes[pos..];

            let val_parsed = match parse_entry(slice_val) {
                Ok(v) => v,
                Err(e) => {
                    return Err(KvError::Corrupted(e));
                }
            };

            let (val_val, used_val) = match val_parsed {
                Some(pair) => pair,
                None => {
                    return Err(KvError::UnexpectedEof);
                }
            };

            pos += used_val;

            let key = match key_val {
                BorrowedValue::Text(s) => Key::Text(s.to_string()),
                BorrowedValue::Integer(i) => Key::Integer(i),
                BorrowedValue::Bool(_) | BorrowedValue::Blob(_) => {
                    return Err(KvError::InvalidKeyType);
                }
            };

            let owned_val = val_val.to_owned();
            store.insert(key, owned_val);
        }

        Ok(store)
    }
}

fn serialize_value(value: &OwnedValue, out: &mut Vec<u8>) {
    let mut payload = Vec::new();
    let tag: TypeTag;

    match value {
        OwnedValue::Integer(x) => {
            tag = TypeTag::Integer;
            let bytes = x.to_le_bytes();
            payload.extend_from_slice(&bytes);
        }
        OwnedValue::Bool(b) => {
            tag = TypeTag::Bool;
            let byte = if *b { 1u8 } else { 0u8 };
            payload.push(byte);
        }
        OwnedValue::Text(s) => {
            tag = TypeTag::Text;
            let bytes = s.as_bytes();
            let len_u64 = bytes.len() as u64;
            let len_bytes = len_u64.to_le_bytes();
            payload.extend_from_slice(&len_bytes);
            payload.extend_from_slice(bytes);
        }
        OwnedValue::Blob(v) => {
            tag = TypeTag::Blob;
            let len_u64 = v.len() as u64;
            let len_bytes = len_u64.to_le_bytes();
            payload.extend_from_slice(&len_bytes);
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

fn deserialize_borrowed(data: &[u8]) -> Result<BorrowedValue<'_>, DecodeError> {
    if data.len() < HEADER_SIZE {
        return Err(DecodeError::SliceTooShortForHeader);
    }

    let header = deserialize_header(data)?;

    let total_len = header.length as usize;
    let needed = LEN_BYTES + total_len;

    if data.len() < needed {
        return Err(DecodeError::EntryTruncated);
    }

    let payload_len = total_len - CHECKSUM_BYTES - TAG_BYTES;
    let payload_start = HEADER_SIZE;
    let payload_end = payload_start + payload_len;

    if data.len() < payload_end {
        return Err(DecodeError::PayloadTruncated);
    }

    let payload = &data[payload_start..payload_end];

    let stored_checksum = header.checksum;
    let tag_byte = header.tag;

    let computed = CRC32.checksum(payload);
    if computed != stored_checksum {
        return Err(DecodeError::ChecksumMismatch {
            computed,
            stored: stored_checksum,
        });
    }

    let tag = match TypeTag::from_u8(tag_byte) {
        Some(t) => t,
        None => return Err(DecodeError::UnknownTypeTag(tag_byte)),
    };

    match tag {
        TypeTag::Integer => {
            if payload.len() < 8 {
                return Err(DecodeError::MissingIntegerPayload);
            }
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&payload[..8]);
            let value = i64::from_le_bytes(buf);
            Ok(BorrowedValue::Integer(value))
        }
        TypeTag::Bool => {
            if payload.is_empty() {
                return Err(DecodeError::MissingBoolPayload);
            }
            let b = payload[0] != 0;
            Ok(BorrowedValue::Bool(b))
        }
        TypeTag::Text => {
            if payload.len() < 8 {
                return Err(DecodeError::MissingTextLength);
            }
            let mut len_buf = [0u8; 8];
            len_buf.copy_from_slice(&payload[0..8]);
            let slen = u64::from_le_bytes(len_buf) as usize;

            if payload.len() < 8 + slen {
                return Err(DecodeError::MissingTextPayload);
            }
            let text_slice = &payload[8..8 + slen];
            let s = str::from_utf8(text_slice)
                .map_err(|_| DecodeError::InvalidUtf8)?;
            Ok(BorrowedValue::Text(s))
        }
        TypeTag::Blob => {
            if payload.len() < 8 {
                return Err(DecodeError::MissingBlobLength);
            }
            let mut len_buf = [0u8; 8];
            len_buf.copy_from_slice(&payload[0..8]);
            let blen = u64::from_le_bytes(len_buf) as usize;

            if payload.len() < 8 + blen {
                return Err(DecodeError::MissingBlobPayload);
            }
            let slice = &payload[8..8 + blen];
            Ok(BorrowedValue::Blob(slice))
        }
    }
}

#[cfg(test)]
impl KvStore {
    pub fn test_get_offset(&self, key: &Key) -> usize {
        self.index[key]
    }

    pub fn test_corrupt_byte(&mut self, offset: usize) {
        self.data[offset] ^= 0xFF;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ktxt(s: &str) -> Key { Key::Text(s.to_string()) }

    #[test]
    fn header_size_matches_const() {
        assert_eq!(HEADER_SIZE, std::mem::size_of::<RawHeader>());
    }

    #[test]
    fn raw_header_roundtrip_single() {
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

        let h2 = deserialize_header(&buf).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn checksum_detects_corruption() {
        let mut kv = KvStore::new();
        kv.insert(ktxt("x"), OwnedValue::Integer(123));

        let off = kv.test_get_offset(&ktxt("x"));
        let corrupt_idx = off + HEADER_SIZE;
        kv.test_corrupt_byte(corrupt_idx);

        let res = kv.get_borrowed(&ktxt("x"));
        assert!(res.is_err());
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
