# CONFIGS.md — Configuration surface table (Phase A)

## Axes the C actually branches on

The library is stateless: there is no init/handle/context, no runtime
option struct, no global flag, and **no `#ifdef`** in `c_src/src/lib.c`
(`grep -c '#if' c_src/src/lib.c` → 0 conditional-compilation branches).
Consequently every "configuration" axis is an *input shape*, enumerated
directly from the branches and arithmetic the source performs:

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| A. entry point | the 6 exported functions; 5 low-level + `doubleneg` (the composed pipeline) | all |
| B. `b == 0` vs `b != 0` | division skipped vs performed | line 57 |
| C. `c % 10` sign & magnitude | exponent `-9..9` → `pow(10, e)` | line 61 |
| D. `a` magnitude/sign | `0`, ±small, `INT_MIN`, `INT_MAX` | line 58 |
| E. double magnitude class | in-range, boundary, out-of-range, ±0, subnormal, ±inf, NaN | line 30 |
| F. `size` | `0`, `1`, `2`, `255`, `256`, large; `< 0` for `create_numeric_buffer` | lines 34, 49 |
| G. needle position | index `0`, middle, last, absent | line 36 |
| H. `search_val` low byte | `0x00`, `0x2A` (42), `0x64` (100), `0x7F`, `0x80`, `0xFF`; supplied as negative / `>255` values | line 33 |
| I. `seed` | `0`, positive, negative, `INT_MAX`/`INT_MIN` (overflow of `seed + i*7`) | line 50 |
| J. negation input | `0` vs non-zero (incl. negative, `INT_MIN`) | lines 43, 75-83 |
| K. `doubleneg` param tuple | which of `param1..4` are zero; signs; extremes; whether byte `100` / `42` land in the buffer | lines 65-150 |
| L. observable channel | return value **and** the exact stdout bytes (18+ `printf` calls) | all of `doubleneg` |

## Rows

Each row is a meaningful combination that the C treats differently. Every
row is exercised with MANY randomized inputs (fixed seed, deterministic
xorshift PRNG) through BOTH `.so`s, comparing return values byte-for-byte —
and, for `doubleneg`, the captured stdout bytes too.

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `process_negation` | zero input | `cfg_negation` | [x] |
| 2  | `process_negation` | non-zero positive, randomized full `i32` range | `cfg_negation` | [x] |
| 3  | `process_negation` | negative incl. `INT_MIN`, `-1` | `cfg_negation` | [x] |
| 4  | `convert_double_to_int` | in-range integral values, randomized | `cfg_conv_in_range` | [x] |
| 5  | `convert_double_to_int` | in-range fractional (truncation toward zero, both signs), randomized | `cfg_conv_fractional` | [x] |
| 6  | `convert_double_to_int` | ±0.0, subnormals, tiny magnitudes | `cfg_conv_tiny` | [x] |
| 7  | `convert_double_to_int` | exact boundaries `±2147483647/8`, ±1 ulp either side | `cfg_conv_boundary` | [x] |
| 8  | `convert_double_to_int` | out-of-range large magnitudes (`2^31 … 1e308`), both signs, randomized | `cfg_conv_out_of_range` | [x] |
| 9  | `convert_double_to_int` | ±inf, quiet/signalling NaN both signs | `cfg_conv_special` | [x] |
| 10 | `convert_double_to_int` | fully random 64-bit patterns reinterpreted as `f64` (covers all classes at once) | `cfg_conv_random_bits` | [x] |
| 11 | `create_numeric_buffer` | `size == 0`, `size == 1`, `size == 2` with seed `0` | `cfg_create_small_sizes` | [x] |
| 12 | `create_numeric_buffer` | `size == 255`, `256`, `257`, `1024`; seed `0` | `cfg_create_sizes` | [x] |
| 13 | `create_numeric_buffer` | positive seeds, randomized × sizes `1..512` | `cfg_create_random` | [x] |
| 14 | `create_numeric_buffer` | negative seeds (negative bytes via truncating `%`), randomized | `cfg_create_random` | [x] |
| 15 | `create_numeric_buffer` | seeds near `INT_MAX`/`INT_MIN` so `seed + i*7` wraps mid-buffer | `cfg_create_overflow_seeds` | [x] |
| 16 | `create_numeric_buffer` | negative `size` (no write) with a pre-filled buffer, randomized | `cfg_create_negative_size` | [x] |
| 17 | `find_value_in_buffer` | needle at index `0` | `cfg_find_positions` | [x] |
| 18 | `find_value_in_buffer` | needle in the middle / first-of-duplicates semantics | `cfg_find_positions` | [x] |
| 19 | `find_value_in_buffer` | needle at the last index (`size-1`) | `cfg_find_positions` | [x] |
| 20 | `find_value_in_buffer` | needle absent | `cfg_find_positions` | [x] |
| 21 | `find_value_in_buffer` | `size == 0`, `1`, `2` | `cfg_find_small_sizes` | [x] |
| 22 | `find_value_in_buffer` | random buffers (all 256 byte values present/absent) × random `search_val` over the full `i32` range | `cfg_find_random` | [x] |
| 23 | `find_value_in_buffer` | `search_val` supplied as negative / `>255` aliasing the same low byte | `cfg_find_aliasing` | [x] |
| 24 | `find_value_in_buffer` | buffer produced by `create_numeric_buffer` (composed pipeline, the shape `doubleneg` uses) | `cfg_find_over_generated_buffer` | [x] |
| 25 | `calculate_with_doubles` | `b == 0`, every exponent `c % 10 ∈ -9..9` | `cfg_calc_b_zero_all_exponents` | [x] |
| 26 | `calculate_with_doubles` | `b != 0`, exponent `0` (pow returns exactly 1.0) | `cfg_calc_exponent_zero` | [x] |
| 27 | `calculate_with_doubles` | `b != 0`, positive exponents `1..9` | `cfg_calc_positive_exponents` | [x] |
| 28 | `calculate_with_doubles` | `b != 0`, negative exponents `-1..-9` (negative `c`) | `cfg_calc_negative_exponents` | [x] |
| 29 | `calculate_with_doubles` | `a == 0` (result `0.0` even though the division runs) | `cfg_calc_a_zero` | [x] |
| 30 | `calculate_with_doubles` | `a`/`b` extremes: `INT_MIN`, `INT_MAX`, `±1`, `b == -1` | `cfg_calc_extremes` | [x] |
| 31 | `calculate_with_doubles` | fully randomized `(a, b, c)` over the whole `i32³` space, bitwise `f64` comparison | `cfg_calc_random` | [x] |
| 32 | `doubleneg` | all-zero params (`b == 0` path + not-found branches) | `doubleneg_valid_configurations` | [x] |
| 33 | `doubleneg` | `param1 == 0`, others non-zero | `doubleneg_valid_configurations` | [x] |
| 34 | `doubleneg` | `param2 == 0` (division skipped, constant `search_byte` loop) | `doubleneg_valid_configurations` | [x] |
| 35 | `doubleneg` | all params `1` (smallest non-zero) | `doubleneg_valid_configurations` | [x] |
| 36 | `doubleneg` | all params negative (negative buffer bytes, negative `% 256`) | `doubleneg_valid_configurations` | [x] |
| 37 | `doubleneg` | mixed signs | `doubleneg_valid_configurations` | [x] |
| 38 | `doubleneg` | params `INT_MAX` / `INT_MIN` (overflow in `param1 + i*param2`, `% 1000` on `INT_MIN`) | `doubleneg_valid_configurations` | [x] |
| 39 | `doubleneg` | `param3` chosen so the exponent is negative → tiny `%e` output | `doubleneg_valid_configurations` | [x] |
| 40 | `doubleneg` | `param3` chosen so the double overflows `int` → `INT_MIN` conversion inside the pipeline | `doubleneg_valid_configurations` | [x] |
| 41 | `doubleneg` | byte `100` absent from the buffer (`direct_search == NULL`) — **unreachable**: the 256 generated bytes are a permutation of all 256 values for every `param1` (see ERRORS.md row 21). Asserted unreachable in BOTH libraries. | `doubleneg_error_paths` | [x] |
| 42 | `doubleneg` | byte `42` absent (a "not found" search line) — **unreachable** for the same reason; the reachable form is covered on `find_value_in_buffer` with `size < 256` | `doubleneg_error_paths` | [x] |
| 43 | `doubleneg` | 200 randomized param tuples, return value + full stdout byte comparison | `doubleneg_valid_configurations` | [x] |
| 44 | composed pipeline | `create_numeric_buffer` → `find_value_in_buffer` → `convert_double_to_int(calculate_with_doubles(..))` driven exactly as `doubleneg` does, randomized, comparing every intermediate | `cfg_pipeline_random` | [x] |
