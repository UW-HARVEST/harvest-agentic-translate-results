# Configuration Surface

`UTIL_createLinePointers` is the only public entry point. The public API has no
runtime options, modes, flags, enums, compile-time feature branches, or element
types. The meaningful valid-input combinations below are the pruned
cross-product of the outer-loop guards, null-terminator scan, terminator-skip
branch, and final line-count check in `../c_src/src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `UTIL_createLinePointers` | `numLines == 0`, `bufferSize == 0`; outer loop is skipped and zero-sized allocation is returned | [x] |
| 2 | `UTIL_createLinePointers` | `numLines == 0`, `bufferSize > 0`; buffer is ignored and outer loop is skipped | [x] |
| 3 | `UTIL_createLinePointers` | one requested non-empty line with no null terminator before `bufferSize`; inner scan stops at the buffer boundary and terminator-skip branch is false | [x] |
| 4 | `UTIL_createLinePointers` | one requested empty line (`buffer[0] == '\0'`); inner scan is skipped and terminator-skip branch is true | [x] |
| 5 | `UTIL_createLinePointers` | one requested line terminated before the end with additional bytes/lines remaining; requested count stops the outer loop and remainder is ignored | [x] |
| 6 | `UTIL_createLinePointers` | multiple requested non-empty lines separated by nulls, with a final non-null-terminated line ending exactly at `bufferSize` | [x] |
| 7 | `UTIL_createLinePointers` | multiple requested lines containing leading/consecutive nulls (empty lines) | [x] |
| 8 | `UTIL_createLinePointers` | multiple requested lines whose final line ends in a null at the final buffer byte; terminator skip advances `pos` exactly to `bufferSize` | [x] |
| 9 | `UTIL_createLinePointers` | requested line count is smaller than the number of line starts present; unrequested trailing data is ignored | [x] |
