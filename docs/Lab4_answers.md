# Questions About Safe and Unsafe Parsers

**Question:
Compare your two parsers: The previous safe one, and the new unsafe one, and discuss
their benefits/drawbacks.**

## Safe vs Unsafe Parser

### Safe Parser:
**Pros (Benefits):**

- Memory safe: Rule of One. Compiler guarantees that no memory corruption happens
- Easy to read
- Easy to maintain: Future changes are safer

**Cons (Drawbacks):**

- Slightly slower
- More code

### Unsafe Parser:

**Pros (Benefits):**

- Very fast
- Less code

**Cons (Drawbacks):**

- Easy to break: Small changes can easily introduce bugs
- Hard to debug: Errors often appear as undefined behavior

### Conclusion

The safe parser is easier to understand, safer to use and easier to maintain. So we prefer it as students.
The unsafe parser can be useful for advanced developers and performance critical code, but it is harder to debug and easier to misuse.

We decide the safe parser as the better and more appropriate solution.