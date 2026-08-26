# CONFIGS.md — Configuration-surface table (Phase A, gate for Phase B)

Derived mechanically from `c_src/include/driver.h`, `c_src/src/driver.c`,
`c_src/CMakeLists.txt` and `Cargo.toml`.

## Axis 1 — build-time configuration

| source | configuration knobs found |
|--------|---------------------------|
| `Cargo.toml` | **no `[features]` section at all** → the only feature combination is the empty one (`--no-default-features`, identical to default) |
| `c_src/CMakeLists.txt` | no `option()`, no `add_definitions`, no `target_compile_definitions`, no conditional sources. Single TU `src/driver.c`; only flag is `-fno-strict-aliasing` |
| `driver.c` / `driver.h` | no `#ifdef` other than the `DRIVER_H_` include guard |

⇒ **Exactly one build configuration exists.** Feature-combination enumeration for
Phase D is the single set `{}` (empty feature set = default).

## Axis 2 — runtime options / modes / flags

Grepped the public header and every `if`/`switch`/`?:`/global in the C:

* public entry points in `driver.h`: **1** — `void driver(int x)`
* internal functions: **1** — `static void print_hex(unsigned char *p, int len)`
* mutable globals / statics / setters / init-or-context structs: **0**
* runtime flags, modes, enums: **0**
* branches on any flag: **0** (the sole branch in the library is `i < len`)

⇒ There is **no runtime option axis**. The configuration surface is therefore the
cross-product of {the 2 entry points reachable across the FFI boundary} × {the
input shapes the code distinguishes}.

## Axis 3 — entry points, including the lowest level

| entry point | reachable across FFI? | how exercised |
|-------------|----------------------|---------------|
| `driver` (public, exported `T`) | yes | dlsym'd from both `.so` files |
| `print_hex` (`static`, not in `.dynsym`) | no — file-local in C, private in Rust | exercised *through* `driver`, which is the only caller; its `len` is always `sizeof(int)`; its `p` always points at the 4-byte local copy. All of its behaviour (loop count, `%02x` promotion, trailing newline) is observed via `driver`'s byte output. |

## Axis 4 — input shapes the C actually distinguishes

`driver`'s single `int` argument is copied with `memcpy` into `char raw[4]` and
dumped byte-by-byte, so the shapes the code distinguishes are properties of the
**object representation**:

* per-byte value class: `0x00`, `0x01..0x0f` (zero-padding path of `%02x`),
  `0x10..0x7f`, `0x80..0xff` (unsigned-char→int promotion path)
* byte position (0..3) — exposes host byte order (little-endian on x86-64)
* whole-value boundaries: `0`, `1`, `-1`, `INT_MAX`, `INT_MIN`, `INT_MAX-1`,
  `INT_MIN+1`, powers of two, `0x0f0f0f0f`/`0xf0f0f0f0` patterns
* sign: non-negative vs negative
* call multiplicity: one call vs many calls in sequence (each appends exactly
  `8 hex digits + "\n"`; no separator state carried between calls)
* argument register width: only the low 32 bits are the `int` (see ERRORS.md E8)

## Configuration-surface table

One row per meaningful combination the C treats differently. Every row is driven
with many randomized inputs (fixed seed, deterministic xorshift PRNG) except
where the row is a fixed boundary set.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|-----|
| C1 | `driver` | no options (none exist); `x == 0` — all four bytes `0x00`, full zero-padding path | [x] |
| C2 | `driver` | `x == 1` — smallest positive; low byte `0x01` in padding path, other three `0x00` | [x] |
| C3 | `driver` | `x == -1` — all four bytes `0xff`, promotion path in every position | [x] |
| C4 | `driver` | whole-value boundaries: `INT_MAX`, `INT_MAX-1`, `INT_MIN`, `INT_MIN+1`, `-2`, `2` | [x] |
| C5 | `driver` | randomized small non-negative values `0..=0xff` (only byte 0 varies, bytes 1-3 are `0x00`) | [x] |
| C6 | `driver` | randomized values in `0x100..=0xffff` (bytes 0-1 vary, bytes 2-3 `0x00`) | [x] |
| C7 | `driver` | randomized values across the full 32-bit range, uniformly random bit patterns (all four bytes vary, mixed classes) | [x] |
| C7b | `driver` | wide sweep of the whole 2^32 space compared with batch captures: a strided walk (stride `0x9e3779b9`, coprime with 2^32, so no value repeats) plus a large uniform random sample — 16384 inputs each by default | [x] |
| C8 | `driver` | randomized **negative** values only (byte 3 always in `0x80..0xff`) | [x] |
| C9 | `driver` | single byte set to `0x00..=0xff` in byte position 0, others `0x00` — exhaustive 256 values (per-byte value class × position 0) | [x] |
| C10 | `driver` | single byte set to `0x00..=0xff` in byte position 1, others `0x00` — exhaustive | [x] |
| C11 | `driver` | single byte set to `0x00..=0xff` in byte position 2, others `0x00` — exhaustive | [x] |
| C12 | `driver` | single byte set to `0x00..=0xff` in byte position 3, others `0x00` — exhaustive (covers sign bit) | [x] |
| C13 | `driver` | all four bytes equal to the same value `v`, `v` in `0x00..=0xff` — exhaustive (byte-order-insensitive control, isolates per-byte formatting) | [x] |
| C14 | `driver` | byte-order discrimination: values whose four bytes are all distinct and non-`0x00` (e.g. `0x04030201`), randomized — detects endianness/ordering mistakes | [x] |
| C15 | `driver` | powers of two `1 << k` for `k` in `0..=31` (single bit set, walks every bit and both byte-value classes) | [x] |
| C16 | `driver` | bitwise complements of powers of two `!(1 << k)` for `k` in `0..=31` (single bit clear) | [x] |
| C17 | `driver`, repeated calls | many calls in one capture (randomized sequence, length 1 / 2 / many) — verifies per-call `"\n"` terminator, no cross-call state, identical buffering/flush ordering through libc `stdout` | [x] |
| C18 | `driver` (low-level ABI shape) | called through a `extern "C" fn(u32)` / `fn(u64)` dlsym signature so the raw register content is controlled directly, randomized — verifies the parameter is read as a 32-bit `int` on both sides (pairs with ERRORS.md E8) | [x] |
| C19 | `driver`, output-stream shape | the same randomized inputs with libc `stdout` redirected to a **regular file** (fully buffered) and to a **pipe** — both sides must emit the identical byte stream in both buffering modes | [x] |
| C20 | `driver` (`print_hex` via `driver`) | output length invariant: for every input in every row above, output is exactly 9 bytes (`8` lowercase hex digits + `\n`), lowercase-only alphabet, `len == 4` iterations — the only branch in the library never degenerates | [x] |
| C21 | `driver` | per-byte independence: the exhaustive 4×256 "digits emitted for byte value v at position p" table is built from each library and compared, then arbitrary byte combinations are checked to be exactly the concatenation of those cells — this is what extends the differential result from sampled inputs to all 2^32 (see VERIFICATION.md) | [x] |
| C22 | `driver` | **exhaustive pairwise (2-wise) byte-value coverage**: all 6 byte-position pairs × all 2^16 value combinations × remaining bytes held at `0x00` and at `0xff` = 786,432 inputs, batch-compared | [x] |

All rows verified. Row → test mapping (`tests/differential.rs`):

| rows | test |
|------|------|
| C1, C2, C3 | `c1_zero_all_bytes_00`, `c2_one_smallest_positive`, `c3_minus_one_all_bytes_ff` |
| C4 | `c4_whole_value_boundaries` |
| C5–C8 | `c5_random_small_non_negative`, `c6_random_two_byte_values`, `c7_random_full_32_bit_range`, `c7b_wide_sweep_of_the_whole_input_space`, `c8_random_negative_values` |
| C9–C13 | `c9_…`/`c10_…`/`c11_…`/`c12_exhaustive_byte_position_*`, `c13_exhaustive_all_four_bytes_equal` |
| C14–C16 | `c14_distinct_non_zero_bytes_byte_order`, `c15_powers_of_two`, `c16_complement_of_powers_of_two` |
| C17 | `c17_repeated_calls_in_one_capture` |
| C18 | `c18_abi_argument_width` |
| C19 | `c19_stdout_buffering_modes` |
| C20 | `c20_output_shape_invariants` (plus `assert_shape` on every other row) |
| C21 | `c21_per_byte_independence_extends_coverage_to_all_inputs` |
| C22 | `c22_exhaustive_pairwise_byte_value_coverage` |
