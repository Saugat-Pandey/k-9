# Why “K-9”?

We named this project **K-9** because it behaves a bit like a quick, loyal retrieval dog.  
You throw data at it, and it fetches it back **fast**, **reliably**, and without hesitation.  
Just like a trained K-9 unit, our key-value store stays focused, efficient, and always ready to retrieve.

# Lab 7

## Why everything is stored in a single file

All keys and values are written into one binary file because it matches the internal structure of the store and keeps persistence simple. The in-memory format already uses a sequential byte layout, so writing the same layout directly to disk avoids extra metadata, multiple file handling, and synchronization issues. A single file is easier to parse, less error-prone, and simplifies testing of save/load roundtrips.

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

All numeric fields use little-endian (to_le_bytes), as required.

## Loading From Disk

The loading implementation uses ```std::fs::read```to load the entire file into memory before deserializing the entries. This matches the lab requirement and keeps the reconstruction process simple and efficient.


# How to Run the Code

## Persist and load manually (example)
```rust 
cargo run --example persist_demo
``` 

## Run all tests

```rust 
cargo test 
``` 

The tests create temporary files (e.g., test_store_roundtrip.bin), restore the store from them, and remove them afterwards.

