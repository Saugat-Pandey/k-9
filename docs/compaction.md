Time complexity of the compaction step

The compaction step has linear time complexity O(n), where n is the number of stored entries (or total size of the stored data).
This is because during compaction, every existing key-value pair must be visited and copied once into a new data buffer.

When is a good time to run the compaction step?

Compaction should not be run frequently, because it is expensive.
A good time to run compaction is:

when many entries have been deleted or overwritten

when memory usage becomes too large

before persisting the key-value store to disk

during idle times when performance is less critical

In practice, compaction is used as a maintenance operation to trade execution time for reduced memory usage.