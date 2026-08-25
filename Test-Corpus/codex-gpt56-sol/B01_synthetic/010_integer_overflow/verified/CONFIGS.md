# Configuration Surface

## Build-Time Configurations

`Cargo.toml` defines `default = []` and no named features.
`c_src/CMakeLists.txt` defines no options, cache-controlled branches, or
preprocessor definitions. Therefore there is exactly one valid build
configuration:

| # | Rust features | C configuration | [ ] |
|---|---------------|-----------------|-----|
| 1 | empty set (`--no-default-features`) | default | [x] |

## Runtime Configurations

There are no headers, runtime modes, flags, options, switches, or conditional
compilation branches. The rows below mechanically cover both exported entry
points and the input/result shapes distinguished by signed `char` promotion,
the `%02x` minimum field width, `fscanf` success versus EOF, and consumption of
only one character.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printHexCharLine` | `char` in `0x00..0x0f`; nonnegative promotion and two-column zero padding | [x] |
| 2 | `printHexCharLine` | `char` in `0x10..0x7f`; nonnegative promotion with no added padding | [x] |
| 3 | `printHexCharLine` | `char` in `0x80..0xff`; negative signed-`char` promotion to `int` | [x] |
| 4 | `main` | empty stdin; failed `fscanf` preserves initialized space, then increments it | [x] |
| 5 | `main` | exactly one byte whose incremented result is nonnegative (`0x00..0x7e` or `0xff`) | [x] |
| 6 | `main` | exactly one byte whose incremented result is negative (`0x7f..0xfe`) | [x] |
| 7 | `main` | multiple bytes, first byte whose incremented result is nonnegative; trailing bytes ignored | [x] |
| 8 | `main` | multiple bytes, first byte whose incremented result is negative; trailing bytes ignored | [x] |
