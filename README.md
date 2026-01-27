# Why “K-9”?

We named this project **K-9** because it behaves a bit like a quick, loyal retrieval dog.  
You throw data at it, and it fetches it back **fast**, **reliably**, and without hesitation.  
Just like a trained K-9 unit, our key-value store stays focused, efficient and always ready to retrieve.

# Lab 7

## Why everything is stored in a single file

We store both keys and values in one sequential binary file because this matches the existing in-memory layout.
Entries are serialized as:

```[KeyEntry][ValueEntry][KeyEntry][ValueEntry]...```


This approach avoids complex multi-file synchronization and makes round-trip parsing trivial.
It also simplifies the loading logic, corruption detection and CRC checking.

## Persistence

The key-value store now supports persistent storage.
Using:
```rust 
persist_to_file(&self, path: &str)
``` 

all keys and values are written to one binary file.
Each entry consists of:

- a 13-byte header (length, checksum, tag)
- a payload (Integer, Bool, Text, Blob)

All numeric fields use little-endian (to_le_bytes) as required.

## Loading From Disk

Loading is implemented with:

```rust 
KvStore::load_from_file(path)
``` 

The implementation uses ```std::fs::read``` as required by the lab instructions to load the entire file into memory before parsing it.

The loader sequentially reads:

**1.** a key entry  
**2.** a value entry  
**3.** inserts both into a new store

### Handling missing files

If the file does not exist, we return a new, empty store (recommended in the task description) .
This makes initialization safe and avoids treating first-time startup as an error.

### Handling corrupted files

If:

- a header is incomplete
- a checksum mismatches
- entries are truncated
- an invalid type tag appears

…the loader returns ```KvError::Corrupted```.

## Completed Lab 7 Tasks (Checklist)

According to the given ToDo list , this project implements:

- [x] Persist keys and values to disk  
```persist_to_file```  writes all entries using the same binary format as in memory.

- [X] Discuss strategies for persistence  
Explained in README:
- single file
- explicit persist instead of per-insert
- reasons for simplicity, consistency and performance

- [X] Load keys and values during initialization  
```load_from_file reconstructs```  the entire store from ```[Key][Value]```  pairs.

- [X] Handle I/O errors (missing files & corrupted data)  
missing file → return empty store  
malformed entries → return ```KvError::Corrupted```   

- [X] Demonstrate correct restoration after restart  
Round-trip tests validate that persisted data is restored exactly.

- [X] Automated test for persistence  
```test_store_roundtrip```  creates a file, reloads it and checks correctness.

- [X] Explicitly test corrupted data  
```checksum_detects_corruption```  flips a byte and ensures the loader reports corruption.

# How to Run the Code

## Persist and load manually (example)
```rust 
cargo run --example persist_demo
``` 

## Run all tests

```rust 
cargo test 
``` 

The tests create temporary files (e.g., test_store_roundtrip.bin), restore the store from them and remove them afterwards.

