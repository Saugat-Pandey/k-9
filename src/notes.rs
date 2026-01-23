use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub updated_at: u64,

    #[serde(default)] 
    pub image: Option<Vec<u8>>,
}

#[derive(Clone)]
pub struct NoteMeta {
    pub id: u64,
    pub title: String,
    pub updated_at: u64,
    pub tags: Vec<String>,
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
            updated_at: now_ts(),
            image: None,
        };
        
        let note_key = crate::Key::Integer(id as i64);
        let note_value = crate::OwnedValue::Blob(note_to_bytes(&note));
        self.kv.insert(note_key, note_value);
        
        let next_meta = crate::OwnedValue::Integer((next_id + 1) as i64);
        self.kv.insert(meta_key, next_meta);
        
        Ok(id)
    }

    pub fn update(&mut self, mut note: Note) -> crate::KvResult<()> {
        note.updated_at = now_ts();
        let key = crate::Key::Integer(note.id as i64);
        let value = crate::OwnedValue::Blob(note_to_bytes(&note));
        self.kv.insert(key, value);
        Ok(())
    }

    pub fn delete(&mut self, id: u64) -> crate::KvResult<()> {
        self.kv.delete(&crate::Key::Integer(id as i64));
        Ok(())
    }

    pub fn list_meta(&self) -> crate::KvResult<Vec<NoteMeta>> {
        let mut metas = Vec::new();
        
        for entry in self.kv.iter() {
            if let crate::Key::Integer(_) = entry.key {
                if let crate::BorrowedValue::Blob(bytes) = entry.value {
                    let note = note_from_bytes(bytes)?;
                    metas.push(NoteMeta {
                        id: note.id,
                        title: note.title,
                        updated_at: note.updated_at,
                        tags: note.tags,
                    });
                } else {
                    return Err(crate::KvError::InvalidKeyType);
                }
            }
        }
        
        metas.sort_by_key(|m| m.id);
        Ok(metas)
    }

    pub fn attach_image(&mut self, id: u64, image_path: &str) -> crate::KvResult<()> {
        let mut note = match self.get(id)? {
            Some(n) => n,
            None => return Ok(()),
        };

        let bytes = std::fs::read(image_path)?;
        note.image = Some(bytes);

        self.update(note)
    }
}

pub fn note_to_bytes(note: &Note) -> Vec<u8> {
    bincode::serialize(note).expect("Failed to serialize note")
}

pub fn note_from_bytes(bytes: &[u8]) -> Result<Note, crate::KvError> {
    bincode::deserialize(bytes).map_err(|_| crate::KvError::Corrupted(crate::DecodeError::NoteDecodeFailed))
}

pub fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}


