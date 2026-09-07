# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the C source, the same way `ERRORS.md` is.

## Axes the C code actually branches on

Enumerated from the public header plus every `if` / `switch` / `#ifdef` / loop
condition in the implementation.

**Public entry points** (`c_src/include/driver.h`) — the *complete* set:

| entry point | signature | lowest-level? |
|-------------|-----------|---------------|
| `driver` | `void driver(float x)` | yes — it is the only public function; there is no convenience wrapper layer |
| `print_hex` | `static void print_hex(unsigned char *p, int len)` | internal linkage, **not** reachable from outside the `.so` (confirmed: `nm` shows it as `t`, absent from `nm -D`). Its `len` is always the constant `sizeof(float) == 4` supplied by `driver`, so `len` is not a caller-controlled axis. |

**Runtime options / modes / flags:** none. Grep for `if`, `switch`, `#ifdef`,
`#if`, and comparison operators in `src/driver.c` yields **zero** matches (see
`ERRORS.md`). There is no global state, no init function, no setter, no
environment-variable read, no compile-time option in `CMakeLists.txt` beyond
`-fno-strict-aliasing`. So there are no option axes to cross-product.

**Control flow that does branch:** exactly one construct, the
`for (int i = 0; i < len; i++)` loop in `print_hex`, with `len` fixed at `4`.
It therefore always executes exactly 4 iterations followed by one `printf("\n")`.

**Input shape axes:** the sole input is one 32-bit `float` passed by value. The
code does not inspect its value — it `memcpy`s the object representation and
formats each of the 4 bytes with `%02x`. So the shape axes that the *observable
output* actually distinguishes are:

1. **Per-byte value class** — which drives `%02x`: bytes `0x00`, `0x01..0x0f`
   (needs zero padding), `0x10..0x7f`, `0x80..0xff` (the signed-`char`
   sign-extension trap, since `char raw[]` is cast to `unsigned char *`).
2. **Byte position** — 4 positions, i.e. native (little-endian) byte order of
   the float must be preserved, not swapped.
3. **IEEE-754 class of the pattern** — `+0`/`-0`, subnormal, normal, `±inf`,
   qNaN, sNaN, NaN payloads. These matter not because the C branches on them
   but because a *translation* can lose them (NaN canonicalisation, float
   normalisation) while passing the argument across the FFI boundary.
4. **Sign bit** — top bit of the last (highest-address) byte on little-endian.

**Call-sequence axis:** `driver` writes to the process-wide libc `stdout`
`FILE`. Both `.so`s share that stream, so single-call vs. repeated-call vs.
C/Rust-interleaved sequences are a distinct configuration.

## Configuration table

Every row is exercised in `translation/tests/differential.rs` against **both**
`.so`s loaded with `libloading`, comparing stdout byte-for-byte. Rows marked
"randomized" use a fixed-seed xorshift PRNG (seed `0x2545F4914F6CDD1D`), many
inputs per row.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | exhaustive: all 256 byte values in byte position 0 (`0x000000NN`) | [x] |
| 2 | `driver` | exhaustive: all 256 byte values in byte position 1 (`0x0000NN00`) | [x] |
| 3 | `driver` | exhaustive: all 256 byte values in byte position 2 (`0x00NN0000`) | [x] |
| 4 | `driver` | exhaustive: all 256 byte values in byte position 3 (`0xNN000000`) | [x] |
| 5 | `driver` | randomized: uniform random 32-bit patterns via `f32::from_bits` (covers all IEEE classes incl. NaN/subnormal), 20000 inputs | [x] |
| 6 | `driver` | randomized: normal finite positive floats (random exponent in the normal range, random mantissa), 4000 inputs | [x] |
| 7 | `driver` | randomized: normal finite negative floats (sign bit set), 4000 inputs | [x] |
| 8 | `driver` | randomized: subnormals only (exponent field `0`, mantissa non-zero), both signs, 4000 inputs | [x] |
| 9 | `driver` | randomized: NaN space only (exponent field all-ones, mantissa non-zero) — qNaN and sNaN, both signs, random payloads, 4000 inputs | [x] |
| 10 | `driver` | randomized: patterns built only from bytes `< 0x10` (stresses `%02x` zero padding in all 4 positions), 2000 inputs | [x] |
| 11 | `driver` | randomized: patterns built only from bytes `>= 0x80` (stresses unsigned-byte printing / no sign extension), 2000 inputs | [x] |
| 12 | `driver` | randomized: patterns built from small integer-valued floats produced by casting random `i16`/`i32` to `f32` (a real consumer's typical values) | [x] |
| 13 | `driver` | randomized: floats produced by arithmetic (`a/b`, `sqrt`, `1/x`) so the argument arrives in an xmm register from a computation rather than a constant load | [x] |
| 14 | `driver` | fixed special values: `0.0`, `-0.0`, `1.0`, `-1.0`, `0.5`, `f32::MIN`, `f32::MAX`, `f32::MIN_POSITIVE`, `f32::EPSILON`, `±inf`, canonical qNaN, `π`, `e` | [x] |
| 15 | `driver` | call sequence: single call, then 1000 sequential calls in one capture window (accumulated stdout must match line for line) | [x] |
| 16 | `driver` | call sequence: C and Rust calls interleaved in the same capture window with no intervening flush (shared `stdout` FILE state) | [x] |
| 17 | `driver` | stdout is a **pipe** (fully buffered) vs. a **regular file** — different libc buffering modes for the same shared `FILE *stdout` | [x] |
| 18 | `driver` | `stdout` explicitly set unbuffered (`setvbuf(_IONBF)`) before the calls, so each `printf` writes through immediately | [x] |
| 19 | `driver` | argument passed after `to_ne_bytes`/`from_bits` round trip *and* as a value loaded from a `Vec<f32>` (memory operand rather than immediate) | [x] |
| 20 | `driver` | binary/driver executable comparison | n/a — `CMakeLists.txt` builds only `add_library(driver SHARED ...)` and `Cargo.toml` declares only `[lib] crate-type = ["cdylib"]`. **No binary executable exists in either project**, so there is no stdout-of-binary comparison to make. |
| 21 | `driver` | exhaustive: all 65536 values of the **upper** 16 bits (sign + exponent + high mantissa), x 4 fixed low halves = 262144 inputs | [x] |
| 22 | `driver` | exhaustive: all 65536 values of the **lower** 16 bits, x 6 fixed upper halves spanning the zero / subnormal / normal / inf / qNaN / -inf exponents = 393216 inputs | [x] |
| 23 | `driver` | strided sweep over the **entire** 2^32 pattern space, prime stride 1021 => 4207743 inputs | [x] |
| 24 | `driver` | 2000000 uniform-random patterns, fixed seed | [x] |
| 25 | `driver` | truly exhaustive all 2^32 patterns, chunked | `#[ignore]`d: ~38 GiB of stdout per side, far past the 600 s budget. Rows 1–4 already cover the `%02x` formatting exhaustively (the output is a per-byte function and every byte value is checked in every position), and rows 21–24 cover >6.8 M distinct patterns of the argument pass. |

Total distinct inputs compared through both `.so`s: **> 6.9 million** per profile.

## Feature combinations

`translation/Cargo.toml` has no `[features]` table and no optional dependencies,
so `--no-default-features` and the default build are the *same* configuration.
Verified by `grep -n feature translation/Cargo.toml` → no match. All of the
default, `--no-default-features` and `--all-features` builds are still run by
`run_all.sh`, against **both** the debug and the release cdylib, because the
`#[no_mangle]` export wrapper is a separately-optimised artifact in each.

## Negative control (proof the tests are not vacuous)

Five deliberately-broken Rust `.so`s were built and fed to the same suite via
`DRIVER_RUST_SO`, to confirm the differential harness actually detects
divergence rather than passing trivially:

| mutant | injected bug | result |
|--------|--------------|--------|
| m1 | `to_be_bytes` instead of `to_ne_bytes` (byte order swapped) | 27 tests FAILED |
| m2 | byte formatted as `b as i8 as c_int` (signed-`char` sign extension) | 25 tests FAILED |
| m3 | NaN payloads canonicalised to `f32::NAN` | 13 tests FAILED — precisely the NaN-sensitive rows (5, 9, 11, 13, 15–17, 19, 21, 22, and `err_nan_payloads`, `err_high_bit_bytes`, `err_past_finite_range`) |
| m4 | trailing `printf("\n")` omitted | 27 tests FAILED |
| m5 | `#[no_mangle]` removed, so `driver` is not exported | 28 tests FAILED, including `phase_d_symbol_parity` |

The unmodified translation passes all 31 tests in every configuration.
