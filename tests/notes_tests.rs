use kv_store::notes::NoteStore;
use std::fs;

#[test]
fn test_create_and_get() {
    let test_file = "test_notes_create_get.bin";
    
    // Cleanup vor dem Test
    let _ = fs::remove_file(test_file);
    
    // Test durchführen
    let mut store = NoteStore::open(test_file).expect("Failed to open store");
    
    let id = store.create("A".to_string(), "Hello".to_string())
        .expect("Failed to create note");
    
    let note = store.get(id).expect("Failed to get note")
        .expect("Note should exist");
    
    assert_eq!(note.id, id);
    assert_eq!(note.title, "A");
    assert_eq!(note.body, "Hello");
    
    // Cleanup nach dem Test
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_update_and_get() {
    let test_file = "test_notes_update.bin";
    
    // Cleanup vor dem Test
    let _ = fs::remove_file(test_file);
    
    // Test durchführen
    let mut store = NoteStore::open(test_file).expect("Failed to open store");
    
    let id = store.create("Original Title".to_string(), "Original Body".to_string())
        .expect("Failed to create note");
    
    // Note abrufen und ändern
    let mut note = store.get(id).expect("Failed to get note")
        .expect("Note should exist");
    
    note.title = "Updated Title".to_string();
    note.body = "Updated Body".to_string();
    
    store.update(note).expect("Failed to update note");
    
    // Prüfen ob die Änderungen gespeichert wurden
    let updated_note = store.get(id).expect("Failed to get updated note")
        .expect("Updated note should exist");
    
    assert_eq!(updated_note.id, id);
    assert_eq!(updated_note.title, "Updated Title");
    assert_eq!(updated_note.body, "Updated Body");
    
    // Cleanup nach dem Test
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_delete() {
    let test_file = "test_notes_delete.bin";
    
    // Cleanup vor dem Test
    let _ = fs::remove_file(test_file);
    
    // Test durchführen
    let mut store = NoteStore::open(test_file).expect("Failed to open store");
    
    let id = store.create("To Delete".to_string(), "This will be deleted".to_string())
        .expect("Failed to create note");
    
    // Prüfen dass die Note existiert
    let note = store.get(id).expect("Failed to get note");
    assert!(note.is_some());
    
    // Note löschen
    store.delete(id).expect("Failed to delete note");
    
    // Prüfen dass die Note nicht mehr existiert
    let deleted_note = store.get(id).expect("Failed to get deleted note");
    assert!(deleted_note.is_none());
    
    // Cleanup nach dem Test
    let _ = fs::remove_file(test_file);
}

#[test]
fn test_persist_and_load() {
    let test_file = "test_notes_persist.bin";
    
    // Cleanup vor dem Test
    let _ = fs::remove_file(test_file);
    
    // Test durchführen
    let id;
    {
        let mut store = NoteStore::open(test_file).expect("Failed to open store");
        
        id = store.create("Persistent Note".to_string(), "This should persist".to_string())
            .expect("Failed to create note");
        
        store.save(test_file).expect("Failed to save store");
    }
    
    // Neuen Store öffnen und prüfen ob die Note noch da ist
    let store = NoteStore::open(test_file).expect("Failed to open persisted store");
    
    let loaded_note = store.get(id).expect("Failed to get loaded note")
        .expect("Persisted note should exist");
    
    assert_eq!(loaded_note.id, id);
    assert_eq!(loaded_note.title, "Persistent Note");
    assert_eq!(loaded_note.body, "This should persist");
    
    // Cleanup nach dem Test
    let _ = fs::remove_file(test_file);
}
