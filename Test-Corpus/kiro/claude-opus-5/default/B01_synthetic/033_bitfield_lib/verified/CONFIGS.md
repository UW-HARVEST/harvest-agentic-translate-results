# CONFIGS.md — Configuration-surface table (Phase B gate)

Derived mechanically from the C source and header, the same way `ERRORS.md` is.

## Axes the C code actually branches on

**Runtime options / modes / flags: none.** `grep` for `if`, `switch`, `#ifdef`,
`#if` over `c_src/src/driver.c` and `c_src/include/driver.h` returns only the
header's include guard (see `ERRORS.md`). There is no global state, no
initialisation call, no mode setter, and no conditional compilation.

**Cargo features: none.** `translation/Cargo.toml` declares no `[features]`
table and no optional dependencies, so `--no-default-features` and the default
build are the same single configuration. There is exactly one feature
combination to verify (verified in Phase D).

**Binary executable: none.** `c_src/CMakeLists.txt` builds only
`add_library(driver SHARED src/driver.c)`; there is no driver binary, and
`translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]` with no
`[[bin]]`. The "compare C and Rust stdout of the binary" clause is therefore
vacuous — but stdout *is* the sole observable of this library, so every row
below compares captured stdout byte-for-byte anyway.

**Public entry points (both, including the lowest-level one):**

| entry point | signature | level |
|-------------|-----------|-------|
| `driver`    | `void driver(unsigned int x, unsigned int y, bool b, int z)` | convenience wrapper: builds a `foo_t`, calls `print_foo` |
| `print_foo` | `void print_foo(const foo_t *foo)` | lowest level: formats an already-built `foo_t`. Exercised directly with raw 8-byte images, so bit-field *decoding* is tested independently of bit-field *encoding* |

**Input shapes the code special-cases** (from the bit-field widths and the
`printf` conversions):

* `x` — `unsigned int : 2`: in-range `0..=3`; out-of-range (truncated).
* `y` — `unsigned int : 3`: in-range `0..=7`; out-of-range (truncated).
* `b` — `bool : 1`, passed as a byte: `0`, `1`, other (`& 1`).
* `z` — full-width `int`: `0`, positive, negative, `INT_MIN`, `INT_MAX`.
* `foo_t` image for `print_foo` — byte 0 (packed bit-fields, all 256 values),
  bytes 1–3 (padding), bytes 4–7 (`z`, little-endian).

## Row table (cross-product, pruned to what the C distinguishes)

Every row asserts C and Rust stdout are byte-identical. Rows marked
*randomized* use ≥256 pseudo-random inputs from a fixed-seed
(`0x243F_6A88_85A3_08D3`) SplitMix64 generator; rows marked *exhaustive*
enumerate the whole space.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `driver` | all in-range combinations of `x∈0..=3`, `y∈0..=7`, `b∈{0,1}` with `z=0` — *exhaustive* (64) | `cfg_c1_driver_inrange_exhaustive` | [x] |
| C2 | `driver` | in-range `x`,`y`,`b`; `z` = each boundary value `{0, 1, -1, i32::MIN, i32::MAX, i32::MIN+1, i32::MAX-1}` — *exhaustive* cross-product (448) | `cfg_c2_driver_z_boundaries` | [x] |
| C3 | `driver` | in-range `x`,`y`,`b`; `z` *randomized* over the full `i32` range (2048) | `cfg_c3_driver_z_random` | [x] |
| C4 | `driver` | `x` *randomized* over the full `u32` range (mostly out-of-range → truncation), `y`,`b`,`z` randomized (2048) | `cfg_c4_driver_x_random_full_u32` | [x] |
| C5 | `driver` | `y` *randomized* over the full `u32` range (mostly out-of-range → truncation), `x`,`b`,`z` randomized (2048) | `cfg_c5_driver_y_random_full_u32` | [x] |
| C6 | `driver` | `b` = every one of the 256 byte values × in-range `x`,`y` × boundary `z` — *exhaustive* over `b` | `cfg_c6_driver_b_all_bytes` | [x] |
| C7 | `driver` | all four arguments *randomized* over their full ABI ranges simultaneously (`x`,`y`: `u32`; `b`: `u8`; `z`: `i32`) — the fully-unconstrained shape (4096) | `cfg_c7_driver_all_random` | [x] |
| C8 | `print_foo` | raw image: byte 0 = every value `0..=255` (covers all `x`/`y`/`b` bit patterns *and* the two padding bits 6–7), padding bytes 1–3 = 0, `z` = 0 — *exhaustive* | `cfg_c8_print_foo_byte0_exhaustive` | [x] |
| C9 | `print_foo` | raw image: byte 0 *randomized*, padding bytes 1–3 *randomized* garbage, `z` *randomized* over full `i32` — the low-level entry point driven with a fully arbitrary struct image (4096) | `cfg_c9_print_foo_image_random` | [x] |
| C10 | `print_foo` | raw image: byte 0 randomized, `z` = each `i32` boundary value — *exhaustive* over the boundary set × 64 random byte-0 values | `cfg_c10_print_foo_z_boundaries` | [x] |
| C11 | `driver` → `print_foo` | composed pipeline: call `driver(x,y,b,z)`, then call `print_foo` on the struct image `driver` is specified to build, and assert *both* libraries agree on *both* calls and that the two calls agree with each other (encode→decode round trip) — *randomized* (2048) | `cfg_c11_encode_decode_roundtrip` | [x] |
| C12 | `driver`, `print_foo` | interleaved multi-call sequence in one process (stdout buffering / statefulness: many calls between a single flush) — *randomized* (512 calls per capture, 16 captures) | `cfg_c12_interleaved_multicall_stream` | [x] |

Feature combinations (Phase D): the single combination `--no-default-features`
≡ default. Verified by `./verify_all.sh`.
