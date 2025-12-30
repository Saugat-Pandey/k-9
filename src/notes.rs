use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Note {
    id: u64,
    title: String,
    body: String,
    tags: Vec<String>,
    updated_at: u64,
}

pub struct NoteMeta {
    id: u64,
    title: String,
    updated_at: u64,
    tags: Vec<String>,
}

pub struct NoteStore {
    kv: crate::KvStore,
}

pub fn note_to_bytes(note: &Note) -> Vec<u8> {
    bincode::serialize(note).expect("Failed to serialize note")
}

pub fn note_from_bytes(bytes: &[u8]) -> Result<Note, crate::KvError> {
    bincode::deserialize(bytes).map_err(|_| crate::KvError::Corrupted(crate::DecodeError::PayloadTruncated))
}
