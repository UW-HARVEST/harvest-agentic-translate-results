# CONFIGS.md — Configuration / valid-input surface table

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from
`c_src/include/lib.h` (public API) plus every `if` / `for` / arithmetic branch in
`c_src/src/lib.c`.

## Axis 0 — runtime options / modes / flags

**There are none.** Grepping `c_src/src/lib.c` finds no `static` state, no
globals, no setters, no mode/flag parameters, no `switch`, and no `#ifdef`. Every
function is pure w.r.t. its arguments (`doubleneg` additionally writes stdout).
The "configuration" of this library is therefore entirely the **shape of the
input values**, enumerated below.

## Axis 1 — public entry points

`include/lib.h` declares only `doubleneg` (the one-shot convenience wrapper), but
the `.so` exports six non-`static` functions. Per the instructions the low-level
entry points are driven **directly**, not only through `doubleneg`:

| level | entry point |
|-------|-------------|
| low   | `convert_double_to_int(double) -> int` |
| low   | `find_value_in_buffer(const char*, size_t, int) -> int` |
| low   | `process_negation(int) -> int` |
| low   | `create_numeric_buffer(char*, int, int) -> void` |
| mid   | `calculate_with_doubles(int, int, int) -> double` (calls `pow`) |
| top   | `doubleneg(int, int, int, int) -> int` (composes all five + stdout) |

## Axis 2 — input shapes the C actually special-cases

* `double` classes: `+0.0`, `-0.0`, subnormal, small fractional, exact integer,
  `INT_MAX`/`INT_MIN` boundary, out-of-`int` range, `±INFINITY`, `NaN`
  (`lib.c:30` `cvttsd2si`).
* buffer sizes: `0`, `1`, `2`, `7` (the stride), `255`, `256` (the size
  `doubleneg` hard-codes), `>256`.
* match position: first / middle / last / absent / duplicated (`memchr` returns
  the *first*).
* `search_val` byte domain: `0x00`, `0x2A` (the literal `42` at `lib.c:106`),
  `0x7F`, `0x80`, `0xFF`, plus values needing `& 0xFF` narrowing.
* `int` sign/magnitude classes: `0`, `1`, `-1`, small +/-, `INT_MAX`, `INT_MIN`.
* `b == 0` vs `b != 0` (`lib.c:57`).
* `c % 10` ∈ `-9..=9` — 19 distinct `pow` exponents (`lib.c:61`).
* zero/non-zero pattern of `doubleneg`'s four params: full 2^4 = 16 truth table
  (each param feeds a distinct `!!` at `lib.c:76,81,82,83`).

## Configuration rows

Every row is exercised with **many randomized inputs** from a fixed-seed
SplitMix64 generator (reproducible) in `tests/phase_b_valid.rs` /
`tests/phase_b_doubleneg.rs`, comparing the C `.so` and Rust `.so` byte-for-byte
via `libloading`.

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `convert_double_to_int` | in-range exact integers, both signs, incl. `0.0` / `-0.0` | `cfg01_convert_in_range_exact_integers` | [x] |
| 2  | `convert_double_to_int` | in-range fractional values → truncation **toward zero** (not floor) for positive and negative | `cfg02_convert_in_range_fractional_truncates_toward_zero` | [x] |
| 3  | `convert_double_to_int` | subnormal / tiny magnitudes (`±5e-324`, `±1e-300`) → `0` | `cfg03_convert_subnormal_and_tiny` | [x] |
| 4  | `convert_double_to_int` | at the `int` boundary: `±2147483647.x`, `-2147483648.0`, `2147483647.9999999` | `cfg04_convert_int_boundary_values` | [x] |
| 5  | `convert_double_to_int` | randomized full-domain `f64` bit patterns (all classes mixed, incl. inf/NaN) | `cfg05_convert_random_full_f64_domain` | [x] |
| 6  | `find_value_in_buffer` | `size=1`, byte present (match at index 0 = last = only) | `cfg06_find_size_one` | [x] |
| 7  | `find_value_in_buffer` | `size=2`, match at index 0 vs index 1 (first-match precedence) | `cfg07_find_size_two_first_match_precedence` | [x] |
| 8  | `find_value_in_buffer` | `size=256`, match at first / middle / last index | `cfg08_find_size_256_first_middle_last` | [x] |
| 9  | `find_value_in_buffer` | duplicated target → must return the **lowest** index | `cfg09_find_duplicates_returns_lowest_index` | [x] |
| 10 | `find_value_in_buffer` | target is `0x00` inside a buffer that also contains later `0x00`s | `cfg10_find_nul_byte_target` | [x] |
| 11 | `find_value_in_buffer` | target `0x80`/`0xFF` (high-bit bytes) through the signed-`char` cast | `cfg11_find_high_bit_targets` | [x] |
| 12 | `find_value_in_buffer` | `size` smaller than the buffer, with the only match **beyond** `size` | `cfg12_find_match_beyond_size_limit` | [x] |
| 13 | `find_value_in_buffer` | randomized buffers (lengths `1..=512`) × randomized `search_val` over the full `int` domain | `cfg13_find_randomized_buffers_and_search_vals` | [x] |
| 14 | `process_negation` | `0`, `1`, `-1`, `INT_MAX`, `INT_MIN`, `256`, `0x10000` (low bits zero) | `cfg14_process_negation_named_shapes` | [x] |
| 15 | `process_negation` | randomized full `int` domain | `cfg15_process_negation_random_full_domain` | [x] |
| 16 | `create_numeric_buffer` | `size=1` — single element, seed `0` | `cfg16_create_size_one` | [x] |
| 17 | `create_numeric_buffer` | `size=7` (equals the stride) — verifies the `i*7` progression | `cfg17_create_size_seven_stride` | [x] |
| 18 | `create_numeric_buffer` | `size=256` — one full permutation of all byte values (`gcd(7,256)=1`) | `cfg18_create_size_256_full_permutation` | [x] |
| 19 | `create_numeric_buffer` | `size=512` — two wraps of the `% 256` cycle | `cfg19_create_size_512_two_wraps` | [x] |
| 20 | `create_numeric_buffer` | `size < capacity` — writes exactly `size` bytes, leaves the tail untouched | `cfg20_create_size_below_capacity_leaves_tail` | [x] |
| 21 | `create_numeric_buffer` | negative seeds → negative C remainder → negative `char` | `cfg21_create_negative_seed_signed_char` | [x] |
| 22 | `create_numeric_buffer` | `seed` near `INT_MAX`/`INT_MIN` so `seed + i*7` wraps mid-loop | `cfg22_create_seed_overflow_midloop` | [x] |
| 23 | `create_numeric_buffer` | randomized `(size, seed)`, full byte-array comparison | `cfg23_create_randomized_size_and_seed` | [x] |
| 24 | `calculate_with_doubles` | `b != 0`, exact division (`a` a multiple of `b`), `c % 10 == 0` → `pow(10,0)` | `cfg24_calc_exact_division_zero_exponent` | [x] |
| 25 | `calculate_with_doubles` | `b != 0`, inexact division → full `f64` rounding must match bit-for-bit | `cfg25_calc_inexact_division_rounding` | [x] |
| 26 | `calculate_with_doubles` | `b == 0` guard taken (`result` stays `0.0`), swept over all `c % 10` | `cfg26_calc_zero_divisor_guard_all_exponents` | [x] |
| 27 | `calculate_with_doubles` | all 19 distinct exponents `c % 10 ∈ -9..=9`, `a`/`b` fixed | `cfg27_calc_all_nineteen_exponents` | [x] |
| 28 | `calculate_with_doubles` | all four sign combinations of `(a, b)`, incl. `a == 0` | `cfg28_calc_sign_combinations` | [x] |
| 29 | `calculate_with_doubles` | extremes `a,b ∈ {INT_MAX, INT_MIN, ±1}` × `c ∈ {INT_MAX, INT_MIN, 0}` | `cfg29_calc_extreme_operands` | [x] |
| 30 | `calculate_with_doubles` | randomized `(a, b, c)` full `int` domain, compared on raw `f64` bits | `cfg30_calc_randomized_full_domain` | [x] |
| 31 | `doubleneg` | all-zero params `(0,0,0,0)` — every `!!` false, `b == 0` guard taken | `doubleneg_all_configurations` (label `row31`) | [x] |
| 32 | `doubleneg` | full 2^4 zero/non-zero truth table of `(p1,p2,p3,p4)` | `doubleneg_all_configurations` (label `row32`) | [x] |
| 33 | `doubleneg` | `p2 == 0` specifically (the `calculate_with_doubles` divisor guard inside the pipeline) | `doubleneg_all_configurations` (label `row33`) | [x] |
| 34 | `doubleneg` | `p3` sweeping all 19 `c % 10` exponent values | `doubleneg_all_configurations` (label `row34`) | [x] |
| 35 | `doubleneg` | `p1` sweeping every buffer seed residue `0..255` (each rotates the byte permutation) | `doubleneg_all_configurations` (label `row35`) | [x] |
| 36 | `doubleneg` | extremes: each of `p1..p4` set to `INT_MAX` / `INT_MIN` in turn and all together | `doubleneg_all_configurations` (label `row36`) | [x] |
| 37 | `doubleneg` | randomized `(p1,p2,p3,p4)` over the full `int` domain — **return value** | `doubleneg_all_configurations` (label `row37`) | [x] |
| 38 | `doubleneg` | randomized `(p1,p2,p3,p4)` — **stdout bytes** captured via `dup2` and compared byte-for-byte (covers `%d`, `%e`, `%ld` formatting and GCC's `printf`→`puts` rewrite) | `doubleneg_all_configurations` (label `row38`) | [x] |
| 39 | *(pipeline)* | low-level composition done by hand: `create_numeric_buffer` → `find_value_in_buffer` → `convert_double_to_int(calculate_with_doubles(..))` → `process_negation`, cross-checked between C and Rust at each stage with randomized inputs | `cfg39_low_level_pipeline_stage_by_stage` | [x] |
| 40 | *(cross-library)* | buffer produced by the **C** `create_numeric_buffer` searched by the **Rust** `find_value_in_buffer` and vice-versa (proves the intermediate byte representation, not just the end result, is identical) | `cfg40_cross_library_buffer_and_search` | [x] |
