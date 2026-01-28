# Questions About Serialization Format

**1. Why do we store the total length of the key and value serializations at the start of the
serialized entry? Why not at the end?**  

So we immediately know how many bytes belong to this entry and can skip to the next one easily.
If the length were at the end, we would have to parse the entire entry to know where the next one starts.

**2. Why do we have to add 2 * std::mem::size_of::<u64>() to total_bytes?**  

Because the key length and value length are both stored as u64, and they also take space.
Therefore we need 2 × 8 = 16 bytes for both length fields.

**3. Why does this serialize function not use any type tags?**  

Because the function only works with String keys and values, so the type is already known. This saves memory.

**4. Why do we cast both the key length and value length to u64?**  

usize depends on the system , but u64 is always the same size. u64 is always exactly 64 bit, so it is platform independent.