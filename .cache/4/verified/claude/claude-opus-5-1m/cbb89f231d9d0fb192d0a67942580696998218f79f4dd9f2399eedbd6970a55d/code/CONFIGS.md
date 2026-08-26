# CONFIGS.md — Phase A configuration-surface table

## How this table was derived

`c_src/src/main.c` exposes exactly two entry points (`nm -D` on the C `.so`):

* `int main(void)` — the *composed pipeline*: skip whitespace, run glibc's
  `%f` conversion on stdin, hand the result to `driver`.
* `void driver(float x)` — the *low-level* entry point: `memcpy` the 4 bytes of
  `x` into a `char[4]` and print them with `printf("%02x")` per byte, then
  `printf("\n")`.  (`print_hex` is `static`, so `driver` is the lowest level a
  caller can reach.)

There are **no runtime options**: no `argc`/`argv` use, no environment
variables, no globals, no locale calls, no `#ifdef`s
(`grep -cE '^\s*#\s*(if|ifdef|...)' c_src/src/main.c` → `0`), and no build
options in `c_src/CMakeLists.txt` beyond `-fno-strict-aliasing`.  `Cargo.toml`
declares no `[features]`.

So the configuration axes are the **input shapes** that the code (i.e. glibc's
`%f` state machine plus the float→bytes dump) actually branches on:

* **A. entry point** — `main` (stdin → stdout) / `driver` (FFI, direct float)
* **B. leading whitespace** — none / one / many / each of `' ' \t \n \v \f \r`
  / mixed, incl. crossing newlines
* **C. sign** — absent / `+` / `-`
* **D. radix** — decimal / hex (`0x` / `0X`), which switches the exponent
  character from `e` to `p` and enables `a`–`f` as digits
* **E. shape of the significand** — integer only / `int.frac` / `.frac` /
  `int.` / `"0"` prefix special case
* **F. exponent** — absent / `e`/`E`/`p`/`P` × sign absent/`+`/`-` ×
  small / large / clamped (> 10^6) / very many digits
* **G. special words** — `inf`, `infinity`, `nan`, `nan(payload)`, in every
  letter case
* **H. magnitude class** — ±0 / subnormal / min-normal / ordinary /
  max-normal / overflow / underflow
* **I. significand length** — 1 digit / ≥ 9 digits (> 24-bit) / ≥ 32 hex
  digits (> 128-bit — trips the sticky-bit path) / 10^5 digits
* **J. rounding position** — exact / just below ½ ulp / exactly ½ ulp with an
  even neighbour / exactly ½ ulp with an odd neighbour / just above ½ ulp
* **K. what follows the number** — EOF / whitespace / a second number / a
  character that stops the scan
* **L. raw float bit pattern** (for `driver`) — ±0 / subnormal / normal /
  ±inf / quiet NaN / signalling NaN / negative NaN / arbitrary 32-bit pattern

The rows below are the pruned cross-product: one row per combination the C
actually treats differently.  Every row is exercised with **many randomized
inputs** (fixed seed, see `tests/common/mod.rs::Rng`) against **both** the C
and the Rust artifacts, comparing stdout byte-for-byte.

## Table

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 1  | `driver` (FFI) | L: exhaustive low bit patterns `0x0000_0000..=0x0001_0000` (65 537 values: `+0` and the first block of subnormals) + 80 000 random 32-bit patterns | [x] |
| 2  | `driver` (FFI) | L: named specials — `±0`, `±FLT_MIN`, `±FLT_TRUE_MIN`, `±FLT_MAX`, `±inf`, qNaN, sNaN, `-qNaN`, `-sNaN`, NaN with 3 000 random payloads (both signs) | [x] |
| 3  | `driver` (FFI) | L: every exponent field 0…255 crossed with mantissa `0 / 1 / 0xff / 0x400000 / 0x7ffffe / 0x7fffff`, both signs | [x] |
| 4  | `main` (FFI, `dlsym("main")`) | A: the exported `main` symbol itself, stdin redirected from a file, decimal input | [x] |
| 5  | `main` (process) | B: no leading whitespace, C: no sign, D: decimal, E: integer only, K: EOF — randomized 1–18 digit integers | [x] |
| 6  | `main` (process) | B: each single whitespace byte (`' ' \t \n \v \f \r`) and long mixed runs before the number | [x] |
| 7  | `main` (process) | C: `+` / `-` / none × D: decimal × E: `int.frac` — randomized | [x] |
| 8  | `main` (process) | E: `.frac` (no integer digits) × C: all signs — randomized | [x] |
| 9  | `main` (process) | E: `int.` (trailing point, no fraction digits), with and without exponent | [x] |
| 10 | `main` (process) | E: `"0"` prefix special case — `0`, `00`, `0.5`, `000123`, `00x5`, `0e5` — randomized leading-zero runs | [x] |
| 11 | `main` (process) | F: decimal exponent `e`/`E` × sign absent/`+`/`-` × 1–3 exponent digits — randomized | [x] |
| 12 | `main` (process) | F: decimal exponent with a huge digit count (≥ 7 digits, exercises the clamp at `10^6 + ndigits`), both signs | [x] |
| 13 | `main` (process) | D: hex `0x`/`0X`, E: integer hex digits only, F: no `p` exponent — randomized 1–20 hex digits, mixed letter case | [x] |
| 14 | `main` (process) | D: hex × E: `int.frac` × F: `p`/`P` with sign absent/`+`/`-` — randomized, exponent in −200…200 | [x] |
| 15 | `main` (process) | D: hex × E: `.frac` only (`0x.8p1`) and `int.` (`0x5.p2`) | [x] |
| 16 | `main` (process) | I: hex significand ≥ 32 digits (> 128 bits) so the `mant >> 120` sticky path is taken, incl. ≥ 64 digits | [x] |
| 17 | `main` (process) | H: subnormal region — random values in `2^-149 … 2^-126`, expressed both in decimal and in hex | [x] |
| 18 | `main` (process) | H: the subnormal/normal boundary and the ½-ulp ties there (`0x…p-149`, `0x…​.8p-149`, `1.4e-45`, `7.00649…e-46`) | [x] |
| 19 | `main` (process) | H: overflow / max-normal boundary (`0x1.fffffep127`, `0x1.ffffffp127`, `3.4028235e38`, `3.4028236e38`), both signs | [x] |
| 20 | `main` (process) | J: exact float values, ±1 ulp neighbours, and the exact ½-ulp midpoints of random adjacent float pairs (ties-to-even in both directions) | [x] |
| 21 | `main` (process) | I/J: randomized floats rendered five ways each — shortest round-trip (`{}`), scientific (`{:e}`), `{:.0..23e}` (1–24 significant digits), exact `%a`-style hex literal, exact full decimal expansion | [x] |
| 22 | `main` (process) | I: 200–3 000 digit significands with a random exponent, and 10^5-digit inputs | [x] |
| 23 | `main` (process) | G: `inf` / `infinity` in every letter-case permutation × C: all signs | [x] |
| 24 | `main` (process) | G: `nan` in every letter-case permutation × C: all signs, with and without an `(n-char-sequence)` payload | [x] |
| 25 | `main` (process) | K: valid number followed by whitespace / a second number / a letter / a `,` / `_` / `.` — only the first token is consumed | [x] |
| 26 | `main` (process) | F: exponent character present but with no digits (`1e`, `1e+`, `0x1p-`), and a second exponent char (`1e5e5`, `0x1p1p1`) | [x] |
| 27 | `main` (process) | E/F: a second `.` after the first (`1.5.5`, `1..5`) and a `.` after the exponent (`1e5.5`) | [x] |
| 28 | `main` (process) | property sweep: random strings over the whole alphabet the state machine distinguishes (`0-9 a-f x X p P e E . + - _ , ( ) i n f t y N A`), length 0–8, with/without sign | [x] |
| 29 | `main` (process) | exhaustive sweep: every string of length ≤ 4 over `0x.pe1-+`, with and without a leading `-` | [x] |
| 30 | `main` (process) | random raw byte strings (0–16 bytes, full 0–255 range, including NUL and ≥ 0x80) | [x] |

## Row → test mapping

| rows | test |
|------|------|
| 1-4  | `tests/ffi_driver_diff.rs::ffi_differential_suite` (libloading, both `.so`) |
| 5    | `tests/valid_paths.rs::row05_plain_decimal_integers` |
| 6    | `tests/valid_paths.rs::row06_leading_whitespace` |
| 7    | `tests/valid_paths.rs::row07_sign_times_int_frac` |
| 8    | `tests/valid_paths.rs::row08_leading_point` |
| 9    | `tests/valid_paths.rs::row09_trailing_point` |
| 10   | `tests/valid_paths.rs::row10_leading_zero_special_case` |
| 11   | `tests/valid_paths.rs::row11_decimal_exponent` |
| 12   | `tests/valid_paths.rs::row12_huge_exponent_digit_counts` |
| 13   | `tests/valid_paths.rs::row13_hex_integers` |
| 14   | `tests/valid_paths.rs::row14_hex_with_binary_exponent` |
| 15   | `tests/valid_paths.rs::row15_hex_point_edges` |
| 16   | `tests/valid_paths.rs::row16_very_wide_hex_significands` |
| 17   | `tests/valid_paths.rs::row17_subnormal_range` |
| 18   | `tests/valid_paths.rs::row18_subnormal_normal_boundary` |
| 19   | `tests/valid_paths.rs::row19_overflow_boundary` |
| 20   | `tests/valid_paths.rs::row20_rounding_boundaries` |
| 21   | `tests/valid_paths.rs::row21_many_textual_forms` |
| 22   | `tests/valid_paths.rs::row22_long_significands` |
| 23   | `tests/valid_paths.rs::row23_infinity_words` |
| 24   | `tests/valid_paths.rs::row24_nan_words` |
| 25   | `tests/valid_paths.rs::row25_trailing_content` |
| 26   | `tests/valid_paths.rs::row26_exponent_without_digits` |
| 27   | `tests/valid_paths.rs::row27_repeated_decimal_points` |
| 28   | `tests/valid_paths.rs::row28_random_alphabet_strings` |
| 29   | `tests/valid_paths.rs::row29_exhaustive_short_strings` |
| 30   | `tests/valid_paths.rs::row30_random_raw_bytes` |

Rows 20 and 18/19 use *exact* representations rather than approximations: the
half-way point between two adjacent `f32` values is `(2*mant + 1) * 2^(e-1)`,
which is written both as an exact `0x…p…` literal and as its exact decimal
expansion (`tests/common/mod.rs::exact_decimal` does the bignum work), so
ties-to-even is exercised in both directions with no rounding introduced by the
test itself.

## Supplementary sweeps (outside the Rust test suite)

`tools/` holds standalone differential sweeps used while hunting for
divergences; they compare the two executables directly and are useful for
running far larger volumes than a unit test should:

* `tools/fuzz.py [n] [seed]` — 150 hand-picked edge cases plus `n` randomized
  inputs from 12 generators (bit patterns, `%g`-style renderings, hex floats,
  exact midpoints, garbage ASCII, raw bytes, long digit strings, …).
* `tools/exhaustive.py {core,wide,words}` — exhaustive short-string sweeps.
* `tools/deep_sweep.py {num5,hex4,word4}` — deeper exhaustive sweeps
  (`len <= 5` over `0x.p1e+-`, `len <= 4` over hex/word alphabets).
* `tools/rounding.py [n] [seed]` — decimal/hex rounding-boundary sweep built on
  Python `Fraction`/`Decimal` for exact midpoints.
* `tools/probe.sh <file>` — prints C vs Rust output for a list of inputs.
