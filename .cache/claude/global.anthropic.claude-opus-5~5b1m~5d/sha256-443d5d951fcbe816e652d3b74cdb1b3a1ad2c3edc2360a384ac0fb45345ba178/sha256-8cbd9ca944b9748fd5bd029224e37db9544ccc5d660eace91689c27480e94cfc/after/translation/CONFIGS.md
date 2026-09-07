# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the C source. Enumeration of the axes the C code
actually distinguishes:

**Public entry points** (from `c_src/include/driver.h`): exactly one —
`void driver(int x)`. Lowest-level function in the file is the `static`
`print_hex(unsigned char *p, int len)`, which is *not* exported (confirmed by
`nm -D`); its only call site is `driver`, with `p = &raw` and `len = 16`, so it
is driven through `driver`.

**Runtime options / modes / flags**: none. There is no option struct, no global
state, no `#ifdef`, no `switch`, and no `if` on any input — `driver` is
straight-line code. `grep -cE '#if|switch|else' c_src/src/driver.c` → 0.

**Input shapes the code distinguishes**: the single `int x` is copied verbatim
into `house_t.floors`, so the only meaningful axis is the *bit pattern* of `x`
(and hence which of the 16 output bytes are non-zero and how each byte is
formatted by `%02x`). The remaining struct fields are constants
(`bedrooms = 3`, `bathrooms = 2.0`), so the last 12 output bytes are fixed;
the `%02x` formatting path is nevertheless value-dependent (bytes `< 0x10` are
zero-padded, bytes `>= 0x80` must not be sign-extended — the classic
`char`-vs-`unsigned char` promotion bug), which the byte-class rows target.

**Feature combinations**: `Cargo.toml` has no `[features]`, so the cross-product
has one element (default == `--no-default-features`). Every row is run under
both invocations.

Every row is exercised with **many randomized inputs** from a fixed-seed PRNG
(`SplitMix64`, seed `0x5DEECE66D`) in addition to its named boundary values, and
compares C vs Rust stdout **byte-for-byte** through the FFI boundary.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `driver` | `x = 0` — all-zero input; every low byte formatted as `00` (zero-padding path) | `cfg_c1_zero` | [x] |
| C2 | `driver` | `x` small positive, one digit (`1..=9`) — single-nibble bytes, `%02x` must pad | `cfg_c2_small_positive` | [x] |
| C3 | `driver` | `x` in `0x10..=0xFF` — first byte two nibbles, high bytes zero; includes `0x7F`/`0x80`/`0xFF` (sign-bit-in-byte, promotion bug class) | `cfg_c3_one_byte_range` | [x] |
| C4 | `driver` | `x` in `0x100..=0xFFFF` — two significant little-endian bytes | `cfg_c4_two_byte_range` | [x] |
| C5 | `driver` | `x` in `0x10000..=0xFFFFFF` — three significant bytes | `cfg_c5_three_byte_range` | [x] |
| C6 | `driver` | `x` with all four bytes significant (`0x01000000..=0x7FFFFFFF`), incl. `INT_MAX` | `cfg_c6_four_byte_range` | [x] |
| C7 | `driver` | `x` negative (`-1`, `-2`, `-256`, `-65536`, `INT_MIN`, random negatives) — two's-complement byte pattern, `0xff` bytes, no sign extension in `%02x` | `cfg_c7_negative` | [x] |
| C8 | `driver` | `x` with every byte forced `>= 0x80` (e.g. `0x80808080`, `0xFFFFFFFF as int`) — all four bytes in the high half | `cfg_c8_high_bytes` | [x] |
| C9 | `driver` | uniform random over the *entire* `u32` bit-pattern space reinterpreted as `int` (4096 seeded samples) — value-dependent path sweep | `cfg_c9_random_full_range` | [x] |
| C10 | `driver` | full exhaustive sweep of the low byte `0x00..=0xFF` combined with a random high 24 bits — every distinct `%02x` byte value at offset 0 | `cfg_c10_exhaustive_low_byte` | [x] |
| C11 | `driver` (repeat / sequencing) | the same value called many times in a row, and an interleaved C-call/Rust-call sequence — the composed pipeline (struct init → `memcpy` → `print_hex` loop → trailing `printf("\n")`) must be idempotent and leave no state | `cfg_c11_repeat_and_interleave` | [x] |
| C12 | `driver` (output shape) | invariant check across all rows: output is always exactly 32 lowercase hex chars + one `\n` (33 bytes), i.e. `sizeof(house_t) == 16` and `bedrooms`/`bathrooms` tail bytes `030000000000000000000040` match between C and Rust — verifies struct layout, padding and `double` (IEEE-754) encoding, not just the `floors` field | `cfg_c12_output_shape_and_tail` | [x] |
| C13 | `driver` | one-shot process-level equivalence: a fresh process loads only ONE library and calls `driver` once, for each library — guards against cross-contamination of the shared stdout FILE* in the in-process differential harness | `cfg_c13_fresh_process_each_lib` | [x] |

## Run log

```
cargo test --offline                                  # debug .so   -> 22 passed
cargo test --offline --release                         # release .so -> 22 passed
cargo test --offline --no-default-features [--release]  # 22 passed
cargo test --offline --all-features [--release]         # 22 passed
```

All 13 rows pass byte-for-byte against the C `.so` across the seeded randomized
inputs (~5k `driver` invocations per full run), under every feature combination
(only one exists) and both profiles. Tests are serialized
(`RUST_TEST_THREADS=1`, set in `.cargo/config.toml`) because the fd-1 capture of
libc `printf` output is process-global.
