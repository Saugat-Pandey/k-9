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
