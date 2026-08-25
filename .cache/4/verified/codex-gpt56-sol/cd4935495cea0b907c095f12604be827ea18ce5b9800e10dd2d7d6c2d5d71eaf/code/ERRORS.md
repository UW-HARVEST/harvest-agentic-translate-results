# Error Surface

Derived from every `return NULL` path and its controlling condition in
`c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `UTIL_createLinePointers` | `malloc(numLines * sizeof(const char**)) == NULL` | `NULL` | [x] |
| 2 | `UTIL_createLinePointers` | after scanning while `lineIndex < numLines && pos < bufferSize`, `lineIndex != numLines` | allocated pointer array is freed, then `NULL` | [x] |

There are no assertions, enums, explicit numeric range checks, min/max
constants, or explicit `buffer == NULL` checks in the C source.

Generic FFI boundaries are covered as follows:

- A null buffer with `numLines > 0` and `bufferSize == 0` takes row 2.
- A null buffer with `numLines == 0` is covered with both zero and nonzero
  `bufferSize` by configuration rows 1 and 2.
- Zero line and buffer lengths are covered by configuration row 1.
- An oversized `numLines` that makes `malloc` fail is covered by row 1.
- There are no enum parameters or documented numeric ranges.

A null buffer with both `numLines > 0` and `bufferSize > 0`, or a
`bufferSize` larger than the readable backing allocation, makes the C code
dereference outside a valid object. Those cases have undefined C behavior
rather than an error result and are therefore not rejection rows.
