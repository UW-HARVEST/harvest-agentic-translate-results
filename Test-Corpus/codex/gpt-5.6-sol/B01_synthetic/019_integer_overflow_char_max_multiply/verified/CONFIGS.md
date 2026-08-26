# Configuration-Surface Table

## Build-time configurations

`Cargo.toml` declares no selectable features (`default = []`). The complete
feature powerset therefore has one member:

| # | Cargo feature set | CMake configuration |
|---|-------------------|----------------------|
| 1 | empty set (`--no-default-features`) | default; no options or preprocessor feature switches |

The C source has no public header and no runtime option/state structure. Its
complete externally callable surface is the five symbols reported by
`nm -D`: `printLine`, `printHexCharLine`, `bad`, `good`, and `main`.

## Runtime configurations

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | non-null empty C string | [x] |
| 2 | `printLine` | non-null C string of one or more bytes, varied lengths and non-NUL byte values | [x] |
| 3 | `printHexCharLine` | negative signed `char` (`CHAR_MIN..-1`), promoted and sign-extended for `%02x` | [x] |
| 4 | `printHexCharLine` | nonnegative signed `char` requiring width padding (`0..15`) | [x] |
| 5 | `printHexCharLine` | nonnegative signed `char` already at least two hex digits (`16..CHAR_MAX`) | [x] |
| 6 | `bad` | fixed `data == CHAR_MAX`; positive guard taken and doubled result converted back to `char` | [x] |
| 7 | `good` | composed call: `goodG2B` safe value followed by `goodB2G` guarded `CHAR_MAX` value | [x] |
| 8 | `main` | successful `%d` conversion to zero, with varied whitespace/sign/text representations | [x] |
| 9 | `main` | successful `%d` conversion to a positive nonzero `int`, with varied values and text representations | [x] |
| 10 | `main` | successful `%d` conversion to a negative nonzero `int`, with varied values and text representations | [x] |
