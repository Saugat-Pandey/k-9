# Lab 8 – Indexing, Deletion and Compaction in the K-9 Key–Value Store

This document describes the design and implementation of indexing, deletion, and the compaction routine in the K-9 key–value store, following the requirements of Lab 8.
The focus is on the compaction step, its correctness, complexity, and observed runtime behavior.

### 1. Motivation: Why Compaction Is Necessary

The key–value store uses an append-only `Vec<u8>` as its underlying data structure.
- Overwriting a key appends a new entry and updates the index.
- Deleting a key removes it from the index but leaves the old data in memory.

This design makes `put`, `get`, and `delete` operations efficient, but it introduces a problem:
- The backing `Vec<u8>` grows monotonically.
- Deleted or outdated entries still consume memory.
- Over time, memory usage increases even if the number of live keys stays constant.

**Compaction** solves this by rebuilding the data vector so that it contains only the currently valid entries.

### 2. Overview of the Compaction Algorithm

The compaction routine follows the steps described in the lab instructions:

1. Iterate over all keys in the index.
2. For each key:
    - Locate its corresponding entry in the old data vector using the stored offset.
    - Parse the entry to determine its exact size.
3. Append the entry to a new `Vec<u8>`.
4. Record the new offset in a fresh index.
5. Replace the old data vector and index with the compacted versions.

This guarantees that:
- Only live entries are retained.
- All offsets in the index remain correct.
- The logical content of the store is unchanged.

### 3. Implementation Details

The compaction is implemented in the following method:

```rust
pub fn compact(&mut self) -> KvResult<()> {
    let mut new_data = Vec::new();
    let mut new_index = HashMap::new();

    for (key, &offset) in &self.index {
        let parsed = parse_entry(&self.data[offset..])
            .map_err(KvError::Corrupted)?
            .ok_or(KvError::UnexpectedEof)?;

        let (_value, used_bytes) = parsed;

        let new_offset = new_data.len();

        new_data.extend_from_slice(
            &self.data[offset .. offset + used_bytes]
        );

        new_index.insert(key.clone(), new_offset);
    }

    self.data = new_data;
    self.index = new_index;

    Ok(())
}
```

### Key aspects of the implementation

- **Parsing entries** parse_entry is used to determine how many bytes belong to a single entry. This avoids relying on fixed-size assumptions and guarantees correctness even for variable-length values.

- **Error handling**
    - Corrupted entries are mapped to `KvError::Corrupted`.
    - Unexpected truncation results in `KvError::UnexpectedEof`. This ensures that compaction never silently produces an inconsistent store.

- **Atomic replacement** The old data and index are only replaced after the new versions are fully constructed, preserving consistency.

### 4. Time Complexity of the Compaction Step

Let:

- `n` be the number of keys in the index.
- `S` be the total size of all live entries in bytes.

The compaction step performs:
- One pass over the index: **O(n)**
- One copy of each live entry: **O(S)**

**Overall time complexity: O(n + S)**

In practice, the runtime is dominated by copying memory proportional to the total size of live data.

### 5. When Is a Good Time to Run Compaction?

Compaction is expensive compared to normal operations, so it should not run on every update.

Good moments to trigger compaction include:
- After a large number of deletions or overwrites.
- When the ratio of live data to total data drops below a threshold.
- During maintenance phases, e.g. application startup or shutdown.
- Manually, when memory pressure becomes noticeable.

In this implementation, compaction is an explicit operation, giving the user full control over when the cost is paid.

### 6. Correctness and Data Integrity

After compaction:

- All keys present in the index are still retrievable.
- Deleted keys remain deleted.
- Iterators only see live entries.
- Offsets correctly reference the new `Vec<u8>`.

This confirms that compaction does not change the logical state of the store, only its physical layout.

### 7. Performance Measurements

When running the stress test using the provided `.txt` dataset in release mode, the observed average execution time was:

**Average runtime (now): 6.14734628 seconds**

This includes:

- Parsing the input data,
- Inserting and updating entries,
- Index lookups,
- And the compaction logic itself.

Given the amount of data processed and the linear nature of the compaction algorithm, this runtime is consistent with expectations.

### 8. Overall Conclusion

- The append-only design provides fast inserts and deletes.
- Compaction is essential to reclaim memory and keep the store efficient.
- The implemented compaction routine is:
    - Correct,
    - Linear in time complexity,
    - Explicitly controlled,
    - And integrates cleanly with the existing index.

The current design balances performance, simplicity, and correctness well and fulfills all requirements of Lab 8.