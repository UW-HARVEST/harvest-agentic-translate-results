# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` section and `c_src/CMakeLists.txt` has no
options or conditional source selection. There is exactly one valid feature
combination:

| # | Cargo invocation | CMake configuration | [ ] |
|---|------------------|---------------------|-----|
| B1 | `cargo ... --no-default-features` (empty feature set) | default, with `CMAKE_POSITION_INDEPENDENT_CODE=ON` | [x] |

## Runtime Configurations

The public surface is the complete set reported by `nm -D`: low-level `foo`,
composed `driver`, and stream-oriented `main`. C strings below are valid,
NUL-terminated allocations. A NUL needle is excluded because the C loop
increments past the string terminator and subsequently passes an out-of-bounds
pointer to `strchr`, which has undefined behavior.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `foo` | empty string; nonzero needle | [x] |
| 2 | `foo` | one-byte string; needle absent | [x] |
| 3 | `foo` | one-byte string; needle present once | [x] |
| 4 | `foo` | many-byte string; needle absent | [x] |
| 5 | `foo` | many-byte string; needle present once | [x] |
| 6 | `foo` | many-byte string; needle present repeatedly, interleaved with other bytes | [x] |
| 7 | `foo` | embedded NUL; matching bytes after the first NUL are ignored | [x] |
| 8 | `foo` | high-bit input bytes and high-bit nonzero needle, exercising signed `char` promotion | [x] |
| 9 | `driver` | empty string; fixed `A` and `x` needles both absent | [x] |
| 10 | `driver` | nonempty string containing `A` but no `x` | [x] |
| 11 | `driver` | nonempty string containing `x` but no `A` | [x] |
| 12 | `driver` | nonempty string containing neither fixed needle | [x] |
| 13 | `driver` | mixed string containing repeated `A` and repeated `x` | [x] |
| 14 | `driver` | embedded NUL; fixed-needle matches after the first NUL are ignored | [x] |
| 15 | `main` | immediate EOF; zero input bytes | [x] |
| 16 | `main` | short stdin read, 1 through 999 bytes, no embedded NUL | [x] |
| 17 | `main` | short stdin read with an embedded NUL | [x] |
| 18 | `main` | exactly 1000 input bytes with an embedded NUL | [x] |
| 19 | `main` | more than 1000 input bytes; only the first 1000 bytes are read, with a NUL in that prefix | [x] |

For `main`, exactly 1000 bytes without a NUL is excluded: `fread` overwrites
the entire zero-initialized array and the subsequent `strchr` reads beyond the
array, which has undefined behavior.
