use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub updated_at: u64,
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

impl NoteStore {
    pub fn open(path: &str) -> crate::KvResult<NoteStore> {
        let kv = match crate::KvStore::load_from_file(path) {
            Ok(store) => store,
            Err(crate::KvError::Io(ref e)) if e.kind() == std::io::ErrorKind::NotFound => {
                crate::KvStore::new()
            }
            Err(e) => return Err(e),
        };
        Ok(NoteStore { kv })
    }

    pub fn save(&self, path: &str) -> crate::KvResult<()> {
        self.kv.persist_to_file(path)
    }

    pub fn get(&self, id: u64) -> crate::KvResult<Option<Note>> {
        let key = crate::Key::Integer(id as i64);
        match self.kv.get_borrowed(&key)? {
            Some(crate::BorrowedValue::Blob(bytes)) => {
                let note = note_from_bytes(bytes)?;
                Ok(Some(note))
            }
            Some(_) => Err(crate::KvError::InvalidKeyType),
            None => Ok(None),
        }
    }

    pub fn create(&mut self, title: String, body: String) -> crate::KvResult<u64> {
        let meta_key = crate::Key::Text("__meta_next_id".to_string());
        
        let next_id = match self.kv.get_owned(&meta_key)? {
            Some(crate::OwnedValue::Integer(i)) => i as u64,
            Some(_) => return Err(crate::KvError::InvalidKeyType),
            None => 1,
        };
        
        let id = next_id;
        let note = Note {
            id,
            title,
            body,
            tags: vec![],
            updated_at: 0,
        };
        
        let note_key = crate::Key::Integer(id as i64);
        let note_value = crate::OwnedValue::Blob(note_to_bytes(&note));
        self.kv.insert(note_key, note_value);
        
        let next_meta = crate::OwnedValue::Integer((next_id + 1) as i64);
        self.kv.insert(meta_key, next_meta);
        
        Ok(id)
    }
}

pub fn note_to_bytes(note: &Note) -> Vec<u8> {
    bincode::serialize(note).expect("Failed to serialize note")
}

pub fn note_from_bytes(bytes: &[u8]) -> Result<Note, crate::KvError> {
    bincode::deserialize(bytes).map_err(|_| crate::KvError::Corrupted(crate::DecodeError::NoteDecodeFailed))
}
