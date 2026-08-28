# Configuration Surface

`UTIL_createLinePointers` is the only public entry point. It has no runtime
options, modes, flags, enums, element types, byte-order setting, compile-time
feature branches, or convenience wrappers. The rows below enumerate the valid
input-shape combinations distinguished by the loop and condition branches in
`../c_src/src/lib.c`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `UTIL_createLinePointers` | `numLines == 0`, `bufferSize == 0`; no pointer entries requested | [x] |
| 2 | `UTIL_createLinePointers` | `numLines == 0`, `bufferSize > 0`; buffer contents are ignored | [x] |
| 3 | `UTIL_createLinePointers` | one non-empty line terminated by `NUL` at the buffer end | [x] |
| 4 | `UTIL_createLinePointers` | one non-empty line without a terminating `NUL`; inner scan stops at `bufferSize` | [x] |
| 5 | `UTIL_createLinePointers` | one empty line (`NUL` at offset zero) | [x] |
| 6 | `UTIL_createLinePointers` | one terminated line followed by extra bytes; outer scan stops at `numLines` | [x] |
| 7 | `UTIL_createLinePointers` | many non-empty, `NUL`-terminated lines exactly consuming the buffer | [x] |
| 8 | `UTIL_createLinePointers` | many lines with leading, middle, or trailing empty lines | [x] |
| 9 | `UTIL_createLinePointers` | many lines where the final requested line is unterminated at `bufferSize` | [x] |
| 10 | `UTIL_createLinePointers` | many requested lines followed by additional unrequested lines or bytes | [x] |
