# Error Surface

Mechanically derived from every `return NULL` branch in
`../c_src/src/lib.c`. The source contains no error macros, `return -1`
statements, assertions, enums, explicit null checks, range checks, or min/max
constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `UTIL_createLinePointers` | `malloc(numLines * sizeof(const char**)) == NULL` | returns `NULL` | [x] |
| 2 | `UTIL_createLinePointers` | after scanning, `lineIndex != numLines` because the buffer supplied fewer line starts than requested (including `numLines > 0 && bufferSize == 0`) | frees the allocated pointer array and returns `NULL` | [x] |
