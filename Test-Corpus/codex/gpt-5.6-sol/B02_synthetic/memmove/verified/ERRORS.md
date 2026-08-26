# Error Surface

The shared-library translation unit has no error enum, assertion, range-error
return, or sentinel other than zero. The two distinct inputs reaching its
single rejection condition are listed separately. `main.c` belongs to the
executable driver and its `scanf`/256-byte input validation is not callable
through the shared-library FFI surface.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `process_buffer` | `buffer == NULL` (including nonzero or oversized `length`) | return `0`; do not dereference `buffer` | [x] |
| 2 | `process_buffer` | `buffer != NULL && length == 0` | return `0`; leave storage unchanged | [x] |

There are no enum parameters. `flags` is a `uint32_t` bitmask, and all bits
outside `0x1f` are accepted and ignored. The FFI function documents no maximum
nonzero length; a nonzero length is valid only when the caller supplies the
corresponding writable storage (up to twice the input length when compaction
can expand it).
