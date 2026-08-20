# CONFIGS.md — configuration / valid-input surface of `c_src/src/lib.c`

Derived mechanically from the branches the C actually takes. There is **no**
runtime option, no mode flag, no global state, no `#ifdef` and no build option in
`c_src` (`CMakeLists.txt` only adds `src/lib.c` to a `SHARED` library), so the
configuration axes are entirely **input shape** axes:

* **Axis S** — sign of each of `a,b,c,d`: a negative argument makes `%d` emit an
  extra `'-'`, which changes `count_occurrences(buffer,'-')` (3 dashes for all
  non-negative … 7 dashes for all negative) *and* the buffer contents.
* **Axis F** — the *bit pattern* of `a` reinterpreted as `float` by
  `int_to_float_bits`, which selects `if (f > 0.0f && f < 1000.0f)`:
  `0.0`, `(0,1)`, `[1,1000)`, `>=1000`, negative, `+inf`, `-inf`, `NaN`.
* **Axis W** — decimal width of each argument (1…11 chars), which drives the
  `snprintf` buffer length (8 … 51 bytes, always < 63 so truncation is
  unreachable) and therefore `process_buffer`'s sum.
* **Axis B** — the low byte of `b`, `c`, `d` (zero vs non-zero vs `0xFF`), which
  drives `interpret_as_int` and `complex_iteration`.
* **Axis L** — length/count arguments of the low-level helpers
  (`0`, `1`, `sizeof(int)`, `< strlen`, `== strlen`, `> strlen`, many).
* **Axis N** — needle/target values, including values outside the `char` range
  passed through an `int` parameter (`memchra`'s `int c`).

Entry points. `lib.h` exposes only `memchra2`, but the eight `static` helpers are
the real low-level entry points and are driven **directly** (not only through the
`memchra2` wrapper) via the `itest_*` exports of feature `internal_test_api` on
the Rust side and `tests/cshim/shim.c` (which `#include`s `c_src/src/lib.c`) on
the C side.

Every row below is exercised with **many randomized inputs** (fixed seed
`0x5EED_1234`, deterministic xorshift PRNG in `tests/common/mod.rs`) and compared
byte-for-byte between the C `.so` and the Rust `.so`.

## Public entry point

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|--------------------------------------------|------|-----|
| 1  | `memchra2` | all args non-negative single digit → 3 dashes, `f` subnormal (`(int)f == 0`) | `cfg01_all_positive_small` | [x] |
| 2  | `memchra2` | all args negative → 7 dashes, `f < 0` (float branch skipped) | `cfg02_all_negative` | [x] |
| 3  | `memchra2` | **all 16 sign patterns** of `(a,b,c,d)` × randomized magnitudes | `cfg03_sign_patterns` | [x] |
| 4  | `memchra2` | `a == 0` → `f == 0.0f`, `f > 0` false | `cfg04_a_zero` | [x] |
| 5  | `memchra2` | `a ∈ [1, 0x3F7FFFFF]` → `0 < f < 1` → `(int)f == 0` added | `cfg05_a_float_lt_one` | [x] |
| 6  | `memchra2` | `a ∈ [0x3F800000, 0x4479FFFF]` → `1 <= f < 1000` → non-zero `(int)f` added | `cfg06_a_float_in_range` | [x] |
| 7  | `memchra2` | `a ∈ [0x447A0000, 0x7F7FFFFF]` → `f >= 1000` → branch skipped | `cfg07_a_float_ge_1000` | [x] |
| 8  | `memchra2` | `a` = `+inf` / `-inf` / quiet NaN / signalling NaN bit patterns | `cfg08_a_float_special` | [x] |
| 9  | `memchra2` | low byte of `b,c,d` == 0 (multiples of 256) → `interpret_as_int` sees zero bytes | `cfg09_low_bytes_zero` | [x] |
| 10 | `memchra2` | low byte of `b,c,d` == `0xFF` → `interpreted == 0x00FFFFFF` | `cfg10_low_bytes_ff` | [x] |
| 11 | `memchra2` | decimal-width sweep 1…10 digits in every argument position (buffer 8…51 bytes) | `cfg11_digit_widths` | [x] |
| 12 | `memchra2` | boundary matrix: `{INT_MIN, INT_MAX, 0, -1, 1}` in every position (5^4 = 625 combos) | `cfg12_boundary_matrix` | [x] |
| 13 | `memchra2` | `a+b+c+d` overflows `int` (wrapping sum in `safe_sum_array`) | `cfg13_sum_overflow` | [x] |
| 14 | `memchra2` | full-range random fuzz over all four arguments (20 000 cases, fixed seed) | `cfg14_random_full_range` | [x] |
| 15 | `memchra2` | repeated/identical arguments (buffer with many equal chars, dash-adjacent digits) | `cfg15_repeated_args` | [x] |

## Low-level entry points (`--features internal_test_api`)

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|--------------------------------------------|------|-----|
| 16 | `memchra` | `n ∈ {0, 1, strlen-1, strlen, strlen+k}` over a fixed buffer | `cfg16_memchra_lengths` | [x] |
| 17 | `memchra` | needle absent / once / many / every byte matches | `cfg17_memchra_needle_density` | [x] |
| 18 | `memchra` | `c` outside `char` range (`256`, `0x141`, `-1`, `INT_MIN`, `INT_MAX`) and `c == 0` over a buffer with embedded NULs | `cfg18_memchra_needle_values` | [x] |
| 19 | `memchra` | randomized buffers (len 0…256, bytes `0x00..0xFF`) × randomized `c` | `cfg19_memchra_random` | [x] |
| 20 | `process_buffer` | `len < strlen`, `len == strlen`, `len > strlen` (interior NUL stops the loop) | `cfg20_process_buffer_lengths` | [x] |
| 21 | `process_buffer` | bytes `>= 0x80` → signed-`char` sign extension makes the sum negative | `cfg21_process_buffer_high_bytes` | [x] |
| 22 | `process_buffer` | long buffer whose signed sum wraps around `INT_MAX` | `cfg22_process_buffer_overflow` | [x] |
| 23 | `process_buffer` | randomized non-empty buffers × randomized `len` | `cfg23_process_buffer_random` | [x] |
| 24 | `int_to_float_bits` | `0, 1, -1, INT_MIN, INT_MAX, 0x3F800000, 0x447A0000, 0x7F800000, 0xFF800000, 0x7FC00000` + random bit patterns (compared as raw `u32` bits) | `cfg24_int_to_float_bits` | [x] |
| 25 | `process_strings` | `count ∈ {1,2,3,4,8}` with all / none / some elements matching | `cfg25_process_strings_counts` | [x] |
| 26 | `process_strings` | target empty, target == element, target longer than element, target shorter, target with embedded NUL | `cfg26_process_strings_targets` | [x] |
| 27 | `process_strings` | array with NULL and empty elements interleaved with matches | `cfg27_process_strings_holes` | [x] |
| 28 | `process_strings` | randomized element sets (incl. NULL/empty holes) × randomized targets | `cfg28_process_strings_random` | [x] |
| 29 | `safe_sum_array` | `size ∈ {1,2,4,17,1000}`, all-positive / all-negative / mixed values | `cfg29_safe_sum_shapes` | [x] |
| 30 | `safe_sum_array` | values chosen so the signed sum wraps (`INT_MAX+INT_MAX`, `INT_MIN+INT_MIN`) | `cfg30_safe_sum_overflow` | [x] |
| 31 | `safe_sum_array` | randomized arrays over the full `int` range | `cfg31_safe_sum_random` | [x] |
| 32 | `interpret_as_int` | `len == 4`, `len > 4` (extra bytes ignored), known byte patterns (endianness), all-`0xFF` → `-1` | `cfg32_interpret_shapes` | [x] |
| 33 | `interpret_as_int` | misaligned base pointer (offsets 1, 2, 3 into a buffer) | `cfg33_interpret_unaligned` | [x] |
| 34 | `interpret_as_int` | randomized byte buffers × randomized `len >= 4` | `cfg34_interpret_random` | [x] |
| 35 | `count_occurrences` | 1-char text, needle present/absent, `ch == 0`, `ch == (char)0xFF` (negative `char`) | `cfg35_count_shapes` | [x] |
| 36 | `count_occurrences` | randomized NUL-terminated texts (len 1…128) × randomized needles | `cfg36_count_random` | [x] |
| 37 | `complex_iteration` | `count ∈ {1,4,256}`, negative values, low bytes 0, values XOR-ing to 0 | `cfg37_complex_shapes` | [x] |
| 38 | `complex_iteration` | randomized arrays over the full `int` range | `cfg38_complex_random` | [x] |
| 39 | whole pipeline: `memchra2` **and** every helper on the same inputs | composed end-to-end run cross-checked against the individually-driven helpers (catches composition-only bugs) | `cfg39_pipeline_consistency` | [x] |
| 40 | the `snprintf("test%d-%d-%d-%d", …)` call site | formatted buffer compared byte-for-byte against glibc's `%d`: boundary matrix, decimal-width sweep in every position, 25 000 randomized quadruples | `cfg40_snprintf_formatting` | [x] |

## Build configurations

`Cargo.toml` `[features]`: `internal_test_api` (no `default` feature set), so the
complete feature power-set is:

| combo | `cargo check --all-targets` | Phase B | Phase C | tests run |
|---|---|---|---|---|
| *(none)* — `--no-default-features` | [x] | [x] rows 1–15 | [x] `ERRORS.md` rows 26, 27, 28, 30, 31 (everything reachable through the shipped surface) | 20 |
| `internal_test_api` | [x] | [x] rows 1–40 | [x] `ERRORS.md` rows 1–31 | 71 |

Both combos were additionally run in the `release` profile (`panic = "abort"`,
optimised) and against an **`-O2`-compiled** C side
(`CSHIM_CFLAGS=-O2 cargo test …`) — identical results in every case.

Driver: `./run_verification.sh` (enumerates the feature power set from
`Cargo.toml`, then runs `cargo check --all-targets`, `cargo build`,
`./check_symbols.sh` and `cargo test` for each combination).

`c_src/CMakeLists.txt` has no options, no `option()`, no `target_compile_definitions`
and no build-type-dependent code, so the C side has exactly one configuration.
