# CONFIGS.md — configuration-surface table (Phase B gate)

## Mechanical derivation of the axes

Public API surface, straight from `c_src/include/driver.h`:

```c
void driver(int x);          /* the ONE and ONLY public entry point */
```

Internal (file-`static`, not part of the dynamic symbol table, therefore not
independently callable by a consumer):

```c
static void print_hex(unsigned char *p, int len);
```

Axes the C code actually branches on, grepped out of `c_src/src/driver.c`:

| axis | values the C distinguishes | evidence |
|------|----------------------------|----------|
| runtime options / modes / flags | **none** | no global/static state, no setter, no flag parameter; `grep -cE 'switch\|#if\|if *\('` on the source finds no branch other than the `for` bound |
| conditional compilation | **none** | no `#ifdef` in `driver.c`; only the `DRIVER_H_` include guard in the header |
| input count / arity | exactly 1 scalar `int` | header signature |
| input shape | scalar only — no buffers, no arrays, no counts, no element types, no formats | there is no pointer or length parameter |
| byte order | fixed by the host ABI (little-endian x86-64); the library dumps the raw struct image, so endianness is observable in the output but is not a selectable option | `memcpy(raw, &house, sizeof(house))` then hex dump |
| output width | fixed: `sizeof(house_t)` == 16 bytes -> 32 hex digits + `\n` | `print_hex(..., sizeof(raw))` |
| value-dependent paths | **none** — every one of the 2^32 `int` values takes the identical code path; only the 4 bytes at offset 0 of the dump change | `house.floors = floors;` is the sole use of the parameter |

So the cross-product is small by construction: 1 entry point x 0 options x
1 input shape. What remains to pin down is the **value axis of the single
`int`**, plus the two structural properties of the output (the constant
`bedrooms`/`bathrooms` bytes and the internal padding of `house_t`). The rows
below are that pruned cross-product; each is exercised with many randomized
inputs from a fixed seed, not one hand-picked value.

There is **no binary executable** in this project (`c_src/CMakeLists.txt`
declares only `add_library(driver SHARED ...)`; `translation/Cargo.toml`
declares only `crate-type = ["cdylib"]`), so the "compare C and Rust binary
stdout" item does not apply.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `driver` | scalar `int`, exhaustive small neighbourhood: every value in `-1024..=1024` | [x] |
| C2 | `driver` | scalar `int`, 20000 uniformly random `i32` values, seeded LCG (seed `0x2545F491_4F6CDD1D`), full 32-bit range incl. negatives | [x] |
| C3 | `driver` | scalar `int`, byte-pattern sweep: single-bit values `1<<0 .. 1<<31` and their bitwise complements (exercises every bit position of the `floors` field, incl. the sign bit) | [x] |
| C4 | `driver` | scalar `int`, boundary values: `0`, `1`, `-1`, `i32::MIN`, `i32::MAX`, `i32::MIN+1`, `i32::MAX-1`, `0x7F`, `0x80`, `0xFF`, `0x100`, `0xFFFF`, `0x10000`, `0x00FF00FF`, `-0x7FFFFFFF` | [x] |
| C5 | `driver` | scalar `int`, repeated invocation of the SAME value 64 times in a row (the C reinitialises `house_t house = {0}` each call; verifies no leaked state between calls and identical output every time) | [x] |
| C6 | `driver` | `driver` called in an interleaved C-then-Rust-then-C sequence over a random value stream (exercises the shared `stdio` FILE buffer both libraries write through, i.e. the composed pipeline rather than one isolated call) | [x] |
| C7 | `driver` | output-shape invariants asserted on top of the byte comparison: exactly 33 bytes emitted, 32 lowercase hex digits + `\n`; bytes 8..16 of the decoded dump equal the IEEE-754 image of `2.0`; bytes 4..8 equal `3i32`; bytes 0..4 equal the little-endian `x` — i.e. the padding-free 16-byte `house_t` layout the C produces | [x] |
| C8 | `print_hex` (lowest-level function) | verified NOT independently reachable: it is `static` in C and absent from `nm -D` on both `.so`s, so the only way a consumer drives it is through `driver`. Rows C1-C7 are therefore the full coverage of the low-level path (`len` is always 16, `p` always `&raw`). | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` section, so the default (empty)
feature set is the only combination. `cargo test --release` and
`cargo test --release --no-default-features` are the same configuration; both
were executed and both pass.
