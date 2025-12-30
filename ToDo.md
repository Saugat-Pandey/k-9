NOTE: This file was created using a LLM.
The original draft and the prompt used to generate it can be found here:
[Lab5 Prompt](Lab5_Prompt.md)


## 1. How do lifetimes ensure that `BorrowedEntry` never outlives the `Store`?

Our `Store` internally holds a buffer, for example a `Vec<u8>`, where all entries are stored in binary form.  
The iterator is defined like this:

```rust
pub struct StoreIter<'a> {
    buf: &'a [u8],
    pos: usize,
    index: &'a HashMap<Key, usize>,
}
```

The method

```rust
impl Store {
    pub fn iter(&self) -> StoreIter<'_> {
        StoreIter {
            buf: &self.buf,
            pos: 0,
            index: &self.index,
        }
    }
}
```

ties the lifetime of the iterator to the lifetime of `&self`.  
`BorrowedEntry` itself looks roughly like this:

```rust
pub struct BorrowedEntry<'a> {
    pub key: &'a Key,
    pub value: &'a Value,
}
```

The iterator therefore returns `BorrowedEntry<'a>`, where `'a` is the same lifetime as in `StoreIter<'a>`, and that lifetime is itself bound to the lifetime of `&self`.

Consequences:

- The compiler enforces that neither `StoreIter` nor any `BorrowedEntry` can outlive the `&Store` from which they were created.
- This guarantees that all references inside `BorrowedEntry` always point to valid data inside the internal `Vec<u8>`.
Once the store goes out of scope, the borrowed entries cannot continue to exist.

In short: lifetimes encode the rule in the type system that “these borrowed views logically belong to the Store and die with it.”
This prevents use-after-free at the type level.

## 2. What happens if the store is mutated during iteration? Should this be allowed?

`iter(&self)` takes only an **immutable borrow** of the store.  
As long as you stay within normal Rust rules (without interior mutability such as `RefCell`, `Mutex`, or `UnsafeCell`), the following holds:

- While a `StoreIter<'_>` exists, there is an active `&Store` borrow.
- At the same time, you cannot obtain an `&mut Store`.
- Methods like `put` or `insert` typically require `&mut self`.  
  Because `&self` and `&mut self` cannot coexist, these mutation methods cannot be called while an iteration is active.

In practice, Rust’s borrow checker *automatically prevents* mutations that could invalidate internal references.  
Mutating the store could cause:

- the underlying buffer (`Vec<u8>`) to reallocate,
- offsets used by the iterator to become invalid,
- previously returned `BorrowedEntry` references to become dangling.

If you circumvent Rust’s guarantees through interior mutability, then:

- the buffer might reallocate,
- references inside existing `BorrowedEntry`s would become invalid,
- the iterator could point to corrupted or meaningless data.

**Design conclusion:**  
Iteration is meant to provide a *read-only view* of the current state.  
Mutating the store during iteration should not be allowed, and Rust ensures this naturally through borrowing rules.


## 3. Could we build a `StoreIterMut`? What happens if the value type changes?

In theory, you could imagine something like:

```rust
pub struct StoreIterMut<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

pub struct BorrowedEntryMut<'a> {
    pub key: &'a mut Key,
    pub value: &'a mut Value,
}
```
However, our store’s data is serialized inside a byte buffer (Vec<u8>):
- entries are delineated by headers, type tags, and length fields,
- the iterator reconstructs typed views from these raw bytes.

Allowing mutation through a mutable iterator introduces fundamental problems:

### 1. Changing the type or length corrupts the serialized layout

If you modify a value such that its type tag or byte length changes:

- the stored header no longer matches the actual content,
- the next iteration step might jump to the wrong offset,
- later reads may interpret garbage as valid entries.

This breaks the entire decoding logic.

### 2. Ensuring "safe" mutations would require extreme restrictions

A hypothetical StoreIterMut would have to enforce that:

- the type cannot be changed,
- the serialized length must remain exactly the same,
- no structural metadata is invalidated.

These constraints are unnatural and hard to enforce safely.

### 3. The design philosophy of the store prefers mutation through API operations

Writing new or updated entries should go through well-defined methods like:

- `put(key, value)`
- `delete(key)`
- `compact()` (if implemented)

These methods know how to serialize correctly, update the buffer, and maintain integrity.

**Conclusion:**
A mutable iterator over raw serialized data is not a good fit for this design.
It would be unsafe, restrictive, and error-prone.
The store is meant to be updated through explicit API operations, not by mutating views into the binary buffer.