# Configuration Surface

## Build-time configurations

Neither `Cargo.toml` nor `c_src/CMakeLists.txt` declares a feature, option,
conditional definition, or backend. There is one valid combination:

| # | Cargo feature combination | CMake configuration | [ ] |
|---|---------------------------|---------------------|-----|
| 1 | empty set (`--no-default-features`) | default, PIC enabled | [x] |

## Runtime configurations

There are no public headers, options, modes, flags, element types, byte-order
settings, pointers, lengths, or lower-level project entry points. The full
public API is `main()`. Its data-shape axes are the number of successful `%d`
assignments by `scanf` and the sign/boundary class passed to libc `div`.

Rows with two assignments use many fixed-seed randomized values. The
`INT_MIN / -1` pair is omitted because its C behavior is undefined.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `main` | EOF/empty input: 0 assignments; defaults `(1, 1)` | [x] |
| 2 | `main` | non-numeric first token: 0 assignments; defaults `(1, 1)` | [x] |
| 3 | `main` | one valid integer then EOF: 1 assignment; default divisor `1` | [x] |
| 4 | `main` | one valid integer then invalid second token: 1 assignment; default divisor `1` | [x] |
| 5 | `main` | two positive integers | [x] |
| 6 | `main` | positive numerator and negative nonzero divisor | [x] |
| 7 | `main` | negative numerator and positive divisor | [x] |
| 8 | `main` | two negative integers, excluding `INT_MIN / -1` | [x] |
| 9 | `main` | zero numerator and nonzero divisor | [x] |
| 10 | `main` | integer boundaries (`INT_MIN`, `INT_MAX`, `1`, `-1`) in defined pairs | [x] |
| 11 | `main` | leading/trailing whitespace and mixed scanf whitespace | [x] |
| 12 | `main` | two integers followed by extra tokens; extras ignored | [x] |
