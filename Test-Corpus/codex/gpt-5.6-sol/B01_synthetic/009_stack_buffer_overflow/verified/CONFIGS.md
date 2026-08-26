# Configuration Surface

There are no compile-time C options, Cargo features, public headers, enums, or
runtime mode flags. The sole build configuration is `--no-default-features`
with an empty feature set.

The rows below are derived from all five externally visible entry points and
the C branches on nullness, `fgets` success, the 14-byte input buffer (at most
13 consumed bytes), sign, the array width 10, and the composed two-read flow in
`main`.

| # | entry point(s) | configuration (options set + input shape) | |
|---|---|---|---|
| 1 | `printLine` | non-null NUL-terminated string; empty and nonempty payloads | [x] |
| 2 | `printIntLine` | any C `int`; negative, zero, positive, `INT_MIN`, and `INT_MAX` | [x] |
| 3 | `bad` | parsed index `0..=9`, newline reached before the 14-byte buffer limit | [x] |
| 4 | `bad` | parsed index `0..=9`, EOF terminates a nonempty final line | [x] |
| 5 | `bad` | parsed index `0..=9`, first read consumes exactly 13 bytes without newline | [x] |
| 6 | `bad` | parsed index `10` (one past the array bound); C performs its unchecked write | [x] |
| 7 | `good` | parsed index `0..=9`, newline reached before the 14-byte buffer limit | [x] |
| 8 | `good` | parsed index `0..=9`, EOF terminates a nonempty final line | [x] |
| 9 | `good` | parsed index `0..=9`, first read consumes exactly 13 bytes without newline | [x] |
| 10 | `main` | two newline-terminated valid indices; `argc`/`argv` are ignored | [x] |
| 11 | `main` | first valid line only, then EOF for `bad` | [x] |
| 12 | `main` | empty input: both reads return `NULL` | [x] |
| 13 | `main` | first `fgets` consumes 13 bytes without newline, so `bad` consumes the remainder | [x] |

Error/rejection shapes are enumerated separately in `ERRORS.md`; every table
row in both files must have a differential test before its checkbox is marked.
