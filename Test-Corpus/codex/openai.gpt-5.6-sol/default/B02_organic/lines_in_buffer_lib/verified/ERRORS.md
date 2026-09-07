# Error Surface

Derived from every rejection branch in `../c_src/src/lib.c`. The public header
declares no enums, ranges, min/max constants, assertions, or additional error
sentinels.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `UTIL_createLinePointers` | `malloc(numLines * sizeof(const char**)) == NULL` | returns `NULL` immediately | [x] |
| 2 | `UTIL_createLinePointers` | after scanning, `lineIndex != numLines` (the buffer contains fewer line starts than requested, including `bufferSize == 0 && numLines > 0`) | frees the allocated pointer array and returns `NULL` | [x] |

Generic FFI boundaries without a distinct C rejection branch (null `buffer`,
zero lengths, oversized lengths) are covered separately by the Phase C tests.
There is no enum parameter, so no out-of-range enum value exists to test.
