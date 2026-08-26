# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` declares no
options or conditional definitions. There is exactly one valid configuration:

| # | Rust feature set | C configuration | [ ] |
|---|------------------|-----------------|-----|
| 1 | empty (`--no-default-features --features ''`) | default | [x] |

## Runtime Configurations

The public API has one entry point, `driver(const char *s1, const char *s2)`.
It has no modes or flags. Its only data-shape axis is where the first byte from
the reject string occurs in the source C string. Randomized cases include
one-byte and many-byte strings, repeated bytes, and arbitrary nonzero byte
values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | empty `s1`; empty `s2`; result `0` | [x] |
| 2 | `driver` | empty `s1`; nonempty `s2`; result `0` | [x] |
| 3 | `driver` | nonempty `s1`; empty `s2`; result is `strlen(s1)` | [x] |
| 4 | `driver` | nonempty strings; first byte of `s1` occurs in `s2`; result `0` | [x] |
| 5 | `driver` | nonempty strings; first rejected byte occurs later in `s1`; result is that byte index | [x] |
| 6 | `driver` | nonempty strings; no byte of `s1` occurs in `s2`; result is `strlen(s1)` | [x] |

There are no lower-level public entry points in `driver.h` or in the C dynamic
symbol table.
