# CONFIGS.md — Phase B configuration-surface table

## Axes derived from the C source

The library has **no** runtime option/mode/flag struct, no setters, no global
state, and no `#ifdef` in `src/lib.c`. `CMakeLists.txt` defines no compile
options either. Therefore the configuration surface is entirely made of
**input shapes** the C code branches on, crossed with the **entry point** used.

Entry points (all six exported symbols — the low-level ones are driven directly,
not only through the `doubleneg` convenience wrapper):

* `E1 convert_double_to_int(double) -> int`
* `E2 find_value_in_buffer(const char*, size_t, int) -> int`
* `E3 process_negation(int) -> int`
* `E4 create_numeric_buffer(char*, int, int) -> void`
* `E5 calculate_with_doubles(int, int, int) -> double`
* `E6 doubleneg(int, int, int, int) -> int`  (composed pipeline: E1..E5 + `printf`)

Branch axes the C actually distinguishes:

| axis | values the source distinguishes | source evidence |
|------|--------------------------------|-----------------|
| A1 `double` magnitude class | in-range / `> INT_MAX` / `< INT_MIN` / `±inf` / NaN / subnormal / `-0.0` | `(int)value` cast, line 30 |
| A2 `double` fractional part | integral / positive fraction (trunc down) / negative fraction (trunc up) | truncating cast, line 30 |
| A3 needle presence | present-first / present-middle / present-last / absent / duplicated | `if (result != NULL)`, line 36 |
| A4 `size` shape | 0 / 1 / small / 256 / large; match inside vs. past `size` | `memchr(buffer, target, size)`, line 35 |
| A5 `search_val` byte class | `0x00` / `0x01..0x7F` / `0x80..0xFF` (sign-extension) / `> 0xFF` / negative / `INT_MIN` / `INT_MAX` | `(char)search_val`, line 34 |
| A6 `size` sign for the writer | `< 0` / `0` / `1` / `256` / `> 256` | `for (i = 0; i < size; i++)`, line 49 |
| A7 `seed` sign & overflow | positive / `0` / negative / `INT_MAX` (overflow) / `INT_MIN` | `(seed + i*7) % 256`, line 50 |
| A8 divisor `b` | `0` (branch skipped) / `±1` / other / `INT_MIN` | `if (b != 0)`, line 57 |
| A9 exponent `c % 10` | `0` / `1..9` / `-1..-9` / `INT_MIN`→`-8` / `INT_MAX`→`7` | `pow(10.0, c % 10)`, line 61 |
| A10 `!!x` operand | `0` / positive / negative / `INT_MIN` / `INT_MAX` | lines 44, 78, 82-84, 131 |
| A11 pipeline param combos | zeros / mixed signs / extremes / overflow-inducing | `doubleneg`, lines 66-151 |
| A12 stdout formatting | `%d`, `%e`, `%ld` renderings incl. inf/NaN/`-0.0` | all `printf` calls in `doubleneg` |

Rows below are the pruned cross-product: one row per combination the C treats
differently. Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_D0UB1E`-derived SplitMix64, see `tests/differential.rs`) comparing the
C `.so` and Rust `.so` byte-for-byte.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 1  | E1 | in-range integral doubles, randomized over `[-2^31, 2^31)` | [x] |
| 2  | E1 | in-range with positive fractional part (truncate toward zero) | [x] |
| 3  | E1 | in-range with negative fractional part (truncate toward zero) | [x] |
| 4  | E1 | exact boundaries `INT_MAX`, `INT_MIN`, `±0.0`, `±1.0` | [x] |
| 5  | E1 | just-past boundaries: `2147483647.5`, `2147483648.0`, `-2147483648.5`, `-2147483649.0`, `nextafter` neighbours | [x] |
| 6  | E1 | far out of range: `±1e18`, `±1e300`, `±DBL_MAX` | [x] |
| 7  | E1 | non-finite: `+inf`, `-inf`, `NAN`, `-NaN`, NaN payload variants | [x] |
| 8  | E1 | subnormals & tiny magnitudes: `±5e-324`, `±1e-300`, `-0.0` | [x] |
| 9  | E1 | fully random 64-bit bit patterns reinterpreted as `double` (covers all classes at once) | [x] |
| 10 | E2 | needle present, `size = 256`, position randomized (first/middle/last) | [x] |
| 11 | E2 | needle absent, `size = 256` → `-1` | [x] |
| 12 | E2 | `size = 0` (buffer non-null) → `-1`; and `size = 0`, buffer `NULL` → `-1` | [x] |
| 13 | E2 | `size = 1`, needle matches / does not match | [x] |
| 14 | E2 | duplicated needle — must return the **first** index | [x] |
| 15 | E2 | needle present only *past* `size` (undersized length) → `-1` | [x] |
| 16 | E2 | `search_val` low byte in `0x80..0xFF` (signed-`char` sign extension) | [x] |
| 17 | E2 | `search_val > 0xFF` (`0x141`, `0x1FF`, `65536+b`) — narrowing must still match | [x] |
| 18 | E2 | `search_val` negative (`-1`, `-128`, `-256`) and `INT_MIN`/`INT_MAX` | [x] |
| 19 | E2 | `search_val = 0` on a buffer with and without a `NUL` byte | [x] |
| 20 | E2 | randomized buffer contents × randomized `size` (1..512) × randomized full-`int` `search_val` | [x] |
| 21 | E3 | `0`, `1`, `-1`, `INT_MIN`, `INT_MAX`, randomized full-range ints | [x] |
| 22 | E4 | `size = 256`, `seed` randomized positive → compare all 256 bytes | [x] |
| 23 | E4 | `size = 256`, `seed` negative (negative `%` result stored as `char`) | [x] |
| 24 | E4 | `seed = 0`, `INT_MAX`, `INT_MIN`, `INT_MAX-7` (signed overflow inside the loop) | [x] |
| 25 | E4 | `size = 0` and `size < 0` (`-1`, `INT_MIN`) → buffer must be untouched | [x] |
| 26 | E4 | `size = 1`, `size = 7`, `size = 255`, `size = 4096` (> 256, wraps the `% 256` cycle many times) | [x] |
| 27 | E4 → E2 | composed: fill with E4 then search with E2 over every byte value `0..255` | [x] |
| 28 | E5 | `b == 0` branch (division skipped, `pow` still applied), `a`/`c` randomized | [x] |
| 29 | E5 | `b != 0`, `c % 10 == 0` → exponent `1.0` | [x] |
| 30 | E5 | `b != 0`, `c % 10` in `1..9` (positive exponent) | [x] |
| 31 | E5 | `b != 0`, `c % 10` in `-1..-9` (negative exponent, C truncating `%`) | [x] |
| 32 | E5 | `b = ±1`, `a` extreme (`INT_MIN`, `INT_MAX`) — no integer overflow, `double` math | [x] |
| 33 | E5 | `a = INT_MIN`, `b = -1` (would trap in integer math) | [x] |
| 34 | E5 | `c = INT_MIN` (`% 10 == -8`) and `c = INT_MAX` (`% 10 == 7`) | [x] |
| 35 | E5 | randomized `(a, b, c)` over full `int` range, bitwise `f64` comparison | [x] |
| 36 | E5 | results that overflow to `inf` / underflow to `0.0` (`a` large, `c%10 == 9` / `-9`) | [x] |
| 37 | E5 → E1 | composed: `convert_double_to_int(calculate_with_doubles(a,b,c))` (the exact `doubleneg` pipeline) | [x] |
| 38 | E6 | all params `0` (baseline: every `!!` zero, `b == 0`, byte 100 absent) — return value **and** stdout | [x] |
| 39 | E6 | `param2 = 0` (forces `b == 0` branch *and* constant `search_byte`) | [x] |
| 40 | E6 | one param non-zero at a time (4 rows worth of `!!` combinations) | [x] |
| 41 | E6 | all `!!` combinations of zero/non-zero across the 4 params (16 shapes) | [x] |
| 42 | E6 | `direct_search` branch: the buffer provably contains **every** byte value at `size = 256` (`gcd(7,256)==1`), so `memchr(buffer, 100, 256)` is never NULL — asserted against the C build, then diffed | [x] |
| 43 | E6 | params where byte 100 lands at various offsets (`%ld` offset rendering) | [x] |
| 44 | E6 | every search value is provably found (each is `x % 256`, narrowed to a byte the buffer contains) — asserted against the C build, then diffed | [x] |
| 45 | E6 | params where `converted_int == INT_MIN` (out-of-range `%e` value → `-648`) | [x] |
| 46 | E6 | `param1`/`param2` extremes (`INT_MIN`, `INT_MAX`) → signed overflow in `param1 + i*param2` and in `seed + i*7` | [x] |
| 47 | E6 | negative params (negative `%` results feeding `search_values`) | [x] |
| 48 | E6 | randomized 4-tuples over full `int` range — return value + stdout byte-for-byte | [x] |
| 49 | E6 | randomized small-magnitude 4-tuples (dense coverage of the `%256` space) | [x] |
| 50 | E6 | stdout formatting classes: `%e` for `inf`/`0.0`/negative/very small; `%ld` offset; `%d` for `INT_MIN` | [x] |

## Additional rows found while driving the surface

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| 51 | E4 → E2 (composed) | `doubleneg`'s exact internal sequence replayed through the exports: fill with `param1`, then search `p2%256`, `p3%256`, `p4%256`, `42`, `100` and the ten `(p1 + i*p2) % 256` bytes, over randomized `int` 4-tuples | [x] |
| 52 | E2 | buffer larger than `INT_MAX` so `(int)((char*)result - buffer)` truncates: match at `INT_MAX`, at `INT_MAX+1` (wraps to `INT_MIN`), at `INT_MAX+5`, and absent over a >2 GiB range | [x] |
| 53 | E4 | `size > INT_MAX / 7` so `i * 7` itself overflows `int` inside the loop (≈292 MiB buffer), across seeds `0, ±1, INT_MAX, INT_MIN` | [x] |
| 54 | E6 | deep randomized fuzz mixing magnitude classes (full-range / `±300` / `±3` / extreme sentinels) — 60,000 iterations verified, 3,000 by default (`DNV_FUZZ_ITERS`) | [x] |

## Where each row is tested

| rows | file | harness |
|------|------|---------|
| 1–37 | `tests/phase_b_configs.rs` | libtest (parallel) |
| 38–50, 51, 54 | `tests/phase_d_pipeline.rs` | `harness = false` (sequential — `doubleneg` stdout needs exclusive fd 1) |
| 52–53 | `tests/phase_e_large.rs` | `harness = false` (multi-gigabyte mappings) |

## Binary / driver

`c_src/CMakeLists.txt` contains a single `add_library(... SHARED src/lib.c)` and
no `add_executable`, so the project builds **no** binary driver. The equivalent
stdout comparison is performed in-process: `doubleneg` is the library's only
stdout-producing entry point, and rows 38–50 + 54 capture file descriptor 1
around both the C and the Rust call and compare the bytes exactly.
