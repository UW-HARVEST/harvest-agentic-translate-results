# Configuration Surface

`Cargo.toml` has no `[features]` table. The complete build-time feature matrix
therefore contains one combination:

| # | default features | named features | check command | status |
|---|------------------|----------------|---------------|--------|
| 1 | disabled | none | `cargo check --no-default-features` | [x] |

The runtime rows below are derived from the public header and the conditions at
lines 12, 17, 23, and 27 of `c_src/src/lib.c`. `UTIL_createLinePointers` is the
only public entry point and there are no runtime options, modes, flags, element
types, formats, or byte-order controls.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|--------|
| 1 | `UTIL_createLinePointers` | `numLines == 0`, `bufferSize == 0`; outer loop is skipped and zero-size allocation is returned | [x] |
| 2 | `UTIL_createLinePointers` | `numLines == 0`, `bufferSize > 0`; input is deliberately ignored | [x] |
| 3 | `UTIL_createLinePointers` | one line whose first byte is `NUL`; inner loop is skipped and `pos` advances past the terminator | [x] |
| 4 | `UTIL_createLinePointers` | one nonempty line ending in `NUL` inside `bufferSize`; inner loop scans bytes and `pos` advances past the terminator | [x] |
| 5 | `UTIL_createLinePointers` | one nonempty line with no `NUL` inside `bufferSize`; inner loop reaches the size boundary and does not increment `pos` again | [x] |
| 6 | `UTIL_createLinePointers` | multiple nonempty `NUL`-separated lines; outer loop iterates many times | [x] |
| 7 | `UTIL_createLinePointers` | multiple lines including adjacent `NUL` bytes; an empty interior line takes the zero-length branch | [x] |
| 8 | `UTIL_createLinePointers` | multiple lines with an unterminated final line at the `bufferSize` boundary | [x] |
| 9 | `UTIL_createLinePointers` | `numLines` is smaller than the number of lines present; only the requested prefix is returned | [x] |
| 10 | `UTIL_createLinePointers` | effective `bufferSize` truncates a larger backing buffer before its next `NUL`; scanning uses only the declared prefix | [x] |
