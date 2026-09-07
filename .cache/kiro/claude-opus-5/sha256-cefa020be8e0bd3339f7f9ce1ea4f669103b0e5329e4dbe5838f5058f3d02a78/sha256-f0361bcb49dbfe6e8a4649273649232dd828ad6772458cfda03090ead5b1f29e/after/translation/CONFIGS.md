# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the C source and the public header, the same way
`ERRORS.md` is derived.

## Mechanical derivation of the axes

Public entry points (`grep -nE '^[a-zA-Z_].*\(' c_src/include/driver.h c_src/src/driver.c`):

| entry point | signature | linkage | in `nm -D`? |
|---|---|---|---|
| `driver` | `void driver(float x)` | external, declared in `include/driver.h` | yes — the ONLY public entry point |
| `print_hex` | `static void print_hex(unsigned char *p, int len)` | `static`, internal | no |

`print_hex` is the lowest-level function. It is `static`, so it is **not**
reachable through the `.so` boundary (confirmed absent from `nm -D` on both
libraries). It is therefore exercised *transitively*, through the only caller
`driver`, in every row below — there is no lower-level entry point to call
directly. `driver` is not a "convenience wrapper" over anything public.

Runtime options / modes / flags the public API can set: **none.**

- no parameters other than `x`
- no setter, no context/handle struct, no global config variable
- `grep -nE '#if|#ifdef|#ifndef'` finds only the `DRIVER_H_` include guard, so
  there is no compile-time configuration axis either
- the Rust `Cargo.toml` declares **no `[features]` table**, so the only feature
  combination is the default one (see "Feature combinations" below)

Axes the code actually branches on:

1. **Loop trip count** (`i < len` in `print_hex`) — fixed at `sizeof(float)` == 4
   by the only call site. Not an input axis, but it fixes output length at 9 bytes.
2. **Per-byte value** fed to `printf("%02x", p[i])`. This is the real axis, and it
   is a per-byte one, not a per-float one:
   - byte `< 0x10` → the `%02x` zero-pad path is taken
   - byte `>= 0x10` → no pad
   - byte `>= 0x80` → the `unsigned char` → `int` default argument promotion must
     be zero-extending, not sign-extending
3. **Input SHAPE = IEEE-754 binary32 class of `x`**, because the bytes come from
   the object representation of `x`: sign bit, biased exponent (0 / all-ones /
   in-between), significand (0 / non-zero). This special-cases zero, subnormal,
   normal, infinity and NaN — and byte ORDER matters, since the bytes are emitted
   in memory order (little-endian on this target: LSB first).
4. **Call multiplicity / interleaving** on the shared process-wide `stdout` FILE
   stream — the only global state the library touches (empty / one / many calls,
   and calls interleaved between the C `.so` and the Rust `.so`).

Rows below are the cross-product of axes 2–4, pruned to the combinations the C
actually distinguishes.

## Configuration-surface table

Every row is driven through the `.so` exports of BOTH libraries with **many
randomized inputs** (`SEED = 0x5EED_1234_ABCD_EF01`, xorshift64\*, fixed for
reproducibility), and stdout captured per call and compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | inputs / row | test | [x] |
|---|----------------|-------------------------------------------|--------------|------|-----|
| 1 | `driver` → `print_hex` | uniformly random 32-bit patterns reinterpreted as `float` (covers all classes in proportion; hits both `%02x` pad paths and byte `>= 0x80`) | 20000 random | `cfg_row01_random_bit_patterns` | [x] |
| 2 | `driver` → `print_hex` | positive normal finite, random exponent+significand | 4000 random | `cfg_row02_positive_normals` | [x] |
| 3 | `driver` → `print_hex` | negative normal finite (sign bit set) | 4000 random | `cfg_row03_negative_normals` | [x] |
| 4 | `driver` → `print_hex` | subnormal, exponent field 0 and significand != 0, both signs | 4000 random | `cfg_row04_subnormals_both_signs` | [x] |
| 5 | `driver` → `print_hex` | both zeros: `+0.0` (`0x00000000`) and `-0.0` (`0x80000000`) | 2 exhaustive | `cfg_row05_both_zeros` | [x] |
| 6 | `driver` → `print_hex` | both infinities: `0x7f800000`, `0xff800000` | 2 exhaustive | `cfg_row06_both_infinities` | [x] |
| 7 | `driver` → `print_hex` | NaN, exponent all-ones and significand != 0, random payload, both signs (quiet + signalling) | 4000 random | `cfg_row07_nans_random_payload` | [x] |
| 8 | `driver` → `print_hex` | small integral values `-2048..=2048` converted via `as f32` (dense low-exponent region) | 4097 exhaustive | `cfg_row08_small_integral_values` | [x] |
| 9 | `driver` → `print_hex` | IEEE boundary constants: `FLT_MAX`, `-FLT_MAX`, `FLT_MIN`, `-FLT_MIN`, `FLT_EPSILON`, smallest/largest subnormal, `1.0`, `-1.0`, `0.5`, `2.0`, `3.0`, `INF`, `-INF`, NaNs, `0x7fffffff`, `0x80000001`, `0xffffffff` | 24 exhaustive | `cfg_row09_boundary_constants` | [x] |
| 10 | `driver` → `print_hex` | **all bytes `< 0x10`** — forces the `%02x` zero-pad on all 4 bytes (random nibble-limited patterns + `0x00000000`, `0x01010101`, `0x0f0f0f0f`) | 2000 random + 3 fixed | `cfg_row10_all_bytes_zero_padded` | [x] |
| 11 | `driver` → `print_hex` | **all bytes `>= 0x80`** — forces the zero-extending `unsigned char` → `int` promotion on all 4 bytes | 2000 random + 2 fixed | `cfg_row11_all_bytes_high_bit_set` | [x] |
| 12 | `driver` → `print_hex` | **exhaustive per-byte coverage**: byte value `b` = 0..=255 placed at each of the 4 positions in turn, rest zero (verifies memory order / little-endian emission AND every possible `%02x` output) | 4 × 256 = 1024 exhaustive | `cfg_row12_each_byte_value_each_position` | [x] |
| 13 | `driver` → `print_hex` | exhaustive sweep of the 8-bit exponent field × sign, significand random (all exponent classes incl. 0 and 255) | 2 × 256 × 8 = 4096 | `cfg_row13_exponent_sweep` | [x] |
| 14 | `driver` → `print_hex` | **call multiplicity**: 1 call, then N=2, 5, 64, 500 calls in one captured stdout window — output must be exactly N × 9 bytes with no state carried across calls | 4 shapes | `cfg_row14_call_multiplicity` | [x] |
| 15 | `driver` → `print_hex` | **interleaved** C-call / Rust-call alternation into the same captured stream, random values, verifying the two `.so`s do not perturb each other's buffered `stdout` | 500 alternations | `cfg_row15_interleaved_c_and_rust` | [x] |
| 16 | `driver` → `print_hex` | **exhaustive low 24-bit-scan**: `x` bit pattern `= k * 0x00010001` for k = 0..=65535, a stride that varies all four bytes together across the whole 32-bit space | 65536 exhaustive | `cfg_row16_strided_full_range_scan` | [x] |
| 17 | `driver` → `print_hex` | output-length invariant under every row above: exactly 8 lowercase hex chars from `[0-9a-f]` + one `\n`, no uppercase, no `0x` prefix, no separator | asserted in every row | `cfg_row17_output_shape_invariant` | [x] |

**Rows: 17. Unchecked: 0.**

## Binary executable

`c_src/CMakeLists.txt` contains a single `add_library(driver SHARED src/driver.c)`
and no `add_executable`; `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` and has no `[[bin]]` / `src/main.rs`. **Neither side
builds a driver binary**, so the "compare binary stdout" gate is not applicable.
The equivalent coverage is obtained by capturing the `.so`s' `stdout` at the fd
level in every row above.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section** and no optional
dependencies, so the complete set of feature combinations is:

| combo | command |
|---|---|
| default (= empty) | `cargo test --release` |
| `--no-default-features` | `cargo test --release --no-default-features` |

Both are run by `run_all.sh`; there is no third configuration to cover.
