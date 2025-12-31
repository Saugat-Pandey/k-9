- [X] Projektstruktur “Core + Apps”  
src/lib.rs enthält den KV-Store als Library  
src/notes.rs ist das Notes-Modul  
pub mod notes; ist eingebunden

- [X] Notes-Datenstrukturen  
Note, NoteMeta, NoteStore existieren

- [X] Note-Serialisierung  
note_to_bytes(note) -> Vec<u8> via bincode  
note_from_bytes(bytes) -> Result<Note, KvError> mit sauberem Mapping auf DecodeError::NoteDecodeFailed

- [X] Error-Handling erweitert  
DecodeError::NoteDecodeFailed ist im KV-Core vorhanden

- [X] NoteStore Grundfunktionen  
NoteStore::open(path) lädt vom KV-Store  
NoteStore::save(path) persistiert  
NoteStore::get(id) lädt Note via Key::Integer(id) und BorrowedValue::Blob

- [X] create(title, body) -> id  
__meta_next_id im KV-Store speichern (Key::Text)  
Note als OwnedValue::Blob unter Key::Integer(id) speichern

- [X] update(note)  
gleiche ID, neuer Blob → insert überschreibt im Index (log-structured “latest wins”)

- [X] delete(id)  
minimal: self.kv.delete(&Key::Integer(id as i64))  

- [X] list_meta() -> Vec<NoteMeta>  
über self.kv.iter() laufen  
nur Integer-Keys berücksichtigen  
Blob dekodieren → Meta bauen (id, title, tags, updated_at)  
sortieren (z.B. nach updated_at oder id)

- [X] Tests für Notes-Layer  
create/get  
update/get  
delete/list  
persist/load roundtrip  
compaction behält latest + entfernt deleted

- [X] CLI-Binary (zum schnellen manuellen Testen)  
src/bin/notes_cli.rs  
Befehle: list, new, show, edit, del, compact  
nutzt NoteStore::open/save/...

### ratatui Integration

- [X] TUI-Binary Skeleton  
src/bin/notes_tui.rs  
Terminal setup/restore (raw mode, alt screen)  
Event loop + q zum Quit

- [X] TUI Layout  
Links: Notizliste (aus list_meta)  
Rechts: Preview (aus get(id))  
Unten: Statusleiste / Hilfe

- [∼] Navigation + Suche  
Up/Down (oder j/k)  [done]
/ Search Mode: filtert nach Titel/Tags

- [] Create/Edit/Delete in der TUI  
n new note (Titel input)  
e edit note (MVP: externer Editor über $EDITOR)  
d delete mit confirm popup  
s save

- [] Compact in TUI  
c compaction + reload