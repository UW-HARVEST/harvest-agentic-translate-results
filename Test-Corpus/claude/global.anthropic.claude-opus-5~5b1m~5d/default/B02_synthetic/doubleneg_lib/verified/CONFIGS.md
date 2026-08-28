# Phase A.3 — Configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Derived mechanically from the
branches the C source actually takes.

## Axes the C code branches on

There are no runtime option/mode/flag setters (no init struct, no context, no
`#ifdef`), so every axis is an **input shape** axis:

| axis | where the C branches on it | distinct states |
|------|----------------------------|-----------------|
| `A1` entry point | `include/lib.h` + `nm -D` | 6: `convert_double_to_int`, `find_value_in_buffer`, `process_negation`, `create_numeric_buffer`, `calculate_with_doubles`, `doubleneg` |
| `A2` double class (`lib.c:30`, `cvttsd2si`) | in-range vs out-of-range vs NaN | `0.0`, `-0.0`, subnormal, small fraction, in-range integral, in-range fractional (truncation toward zero, both signs), `±INT_MAX/INT_MIN` boundary, out-of-range, `±inf`, NaN |
| `A3` `memchr` hit position (`lib.c:35`) | `result != NULL` (`lib.c:36`) | miss, hit at index 0, hit in the middle, hit at index `size-1` |
| `A4` `size` for `memchr` (`lib.c:33`) | `size` argument | `0`, `1`, `2`, `7`, `255`, `256`, `257`, `4096` |
| `A5` needle width (`lib.c:34` `(char)search_val`) | truncation to low byte | `0`, `1`, `42`, `100`, `127`, `128`, `255`, `256`, `300`, `-1`, `-128`, `INT_MIN`, `INT_MAX` |
| `A6` `create_numeric_buffer` `size` (`lib.c:49`) | loop trip count | `<=0`, `1`, `7`, `8`, `36` (one `%256` wrap of the `*7` stride), `255`, `256`, `257`, `1024` |
| `A7` `seed` sign/magnitude (`lib.c:50`) | C `%` truncation ⇒ negative remainders; `int` overflow | `0`, small positive, small negative, `±255`, `±256`, `INT_MAX`, `INT_MIN`, random |
| `A8` divisor (`lib.c:57`) | `if (b != 0)` | `b == 0` (skip), `b != 0` |
| `A9` exponent (`lib.c:61`) | `c % 10` | `0`, `1..9`, `-1..-9`, `10`, `INT_MAX` (`%10 == 7`), `INT_MIN` (`%10 == -8`) |
| `A10` dividend/divisor signs (`lib.c:58`) | `(double)a/(double)b` | `+/+`, `+/-`, `-/+`, `-/-`, `a == 0` (⇒ `±0.0`), `INT_MIN`, `INT_MAX` |
| `A11` `doubleneg` truthiness pattern (`lib.c:76,81-83`) | four independent `!!` | all 16 zero/non-zero combinations of `(param1,param2,param3,param4)` |
| `A12` `doubleneg` observable channel | `printf` × 40+ call sites vs `return result` | stdout bytes **and** return value must both match |

## Pruned cross-product — one row per combination the C treats differently

Every row is exercised with **many randomised inputs** (fixed seed
`0x5EED_1234_ABCD_0001`, SplitMix64) unless the row names exact fixed values,
and is compared C-vs-Rust through `dlsym`'d `.so` exports only.

| #  | entry point(s) | configuration (options set + input shape) | test | ✅ |
|----|----------------|--------------------------------------------|------|----|
| 1  | `process_negation` | exhaustive interesting ints: `0`, `±1`, `±2`, `INT_MIN`, `INT_MAX`, `INT_MIN+1`, `INT_MAX-1` | `cfg_01_negation_fixed` | [x] |
| 2  | `process_negation` | 20 000 random `i32` (full 32-bit range, ~half zero-ish via masking) | `cfg_02_negation_random` | [x] |
| 3  | `convert_double_to_int` | `A2` = exact zeros and subnormals: `0.0`, `-0.0`, `f64::MIN_POSITIVE`, `±5e-324`, `±1e-300` | `cfg_03_cvt_zero_subnormal` | [x] |
| 4  | `convert_double_to_int` | `A2` = in-range **fractional**, both signs, truncation toward zero (`±0.5`, `±1.5`, `±2.9999`, `±1e9+0.5`) | `cfg_04_cvt_fractional` | [x] |
| 5  | `convert_double_to_int` | `A2` = in-range integral across the whole `int` range: every `i32` boundary + 20 000 random `i32` widened to `f64` | `cfg_05_cvt_all_int_values` | [x] |
| 6  | `convert_double_to_int` | `A2` = boundary sweep `2147483647.0 ± {0,0.25,0.5,1}` and `-2147483648.0 ± {0,0.25,0.5,1}` | `cfg_06_cvt_boundary_sweep` | [x] |
| 7  | `convert_double_to_int` | `A2` = 50 000 uniformly random 64-bit **bit patterns** reinterpreted as `f64` (covers inf/NaN/huge/tiny/denormal in one sweep) | `cfg_07_cvt_random_bits` | [x] |
| 8  | `convert_double_to_int` | `A2` = random `f64` scaled into `[-2^40, 2^40]` (straddles the in/out-of-range edge) | `cfg_08_cvt_random_scaled` | [x] |
| 9  | `calculate_with_doubles` | `A8` = `b != 0`, `A9` = `c % 10 == 0` (`c ∈ {0,10,-10,20}`), `A10` all sign combos | `cfg_09_calc_exponent_zero` | [x] |
| 10 | `calculate_with_doubles` | `A8` = `b != 0`, `A9` = `c % 10 ∈ 1..=9` (each value), `A10` all sign combos | `cfg_10_calc_positive_exponents` | [x] |
| 11 | `calculate_with_doubles` | `A8` = `b != 0`, `A9` = `c % 10 ∈ -9..=-1` (each value), `A10` all sign combos | `cfg_11_calc_negative_exponents` | [x] |
| 12 | `calculate_with_doubles` | `A8` = `b == 0`, `A9` swept over all of `-9..=9` (result must be `0.0` with matching sign/bits) | `cfg_12_calc_zero_divisor_all_exponents` | [x] |
| 13 | `calculate_with_doubles` | `A10` = `a == 0` with `b` positive and negative ⇒ `+0.0` vs `-0.0` (bit-exact sign of zero) | `cfg_13_calc_signed_zero` | [x] |
| 14 | `calculate_with_doubles` | `A10` extremes: `a,b ∈ {INT_MIN, INT_MIN+1, -1, 1, INT_MAX}` × `c ∈ {INT_MIN, -1, 0, 1, INT_MAX}` (full cross-product) | `cfg_14_calc_extremes_cross` | [x] |
| 15 | `calculate_with_doubles` | 30 000 fully random `(a,b,c)` triples over the whole `i32` range; compared as raw `u64` bit patterns | `cfg_15_calc_random` | [x] |
| 16 | `create_numeric_buffer` | `A6` = `1`, `A7` = `0` (minimal non-empty) | `cfg_16_create_size_one` | [x] |
| 17 | `create_numeric_buffer` | `A6` ∈ {`1`,`7`,`8`,`36`,`37`,`255`,`256`,`257`,`1024`} × `A7` ∈ {`0`,`1`,`42`,`255`,`256`,`-1`,`-255`,`-256`,`INT_MAX`,`INT_MIN`} (full cross-product, whole buffer compared byte-for-byte plus red-zone canaries) | `cfg_17_create_size_seed_cross` | [x] |
| 18 | `create_numeric_buffer` | `A6`/`A7` random: 5 000 iterations, `size ∈ 0..=2048`, `seed` full `i32` range, canary-guarded buffers | `cfg_18_create_random` | [x] |
| 19 | `create_numeric_buffer` | `A7` = seeds that make `seed + i*7` cross `INT_MAX` mid-loop (`INT_MAX-3`, `INT_MAX-7*100`) — wrapped signed arithmetic | `cfg_19_create_overflow_midloop` | [x] |
| 20 | `find_value_in_buffer` | `A3` = hit at index `0`, `A4` ∈ {`1`,`2`,`256`}, `A5` = exact byte | `cfg_20_find_hit_first` | [x] |
| 21 | `find_value_in_buffer` | `A3` = hit in the middle / hit at `size-1` / miss, `A4` ∈ {`1`,`2`,`7`,`255`,`256`,`257`,`4096`} (full cross-product) | `cfg_21_find_position_size_cross` | [x] |
| 22 | `find_value_in_buffer` | `A5` = needle width sweep `{0,1,42,100,127,128,255,256,300,-1,-128,INT_MIN,INT_MAX}` against a buffer holding **all 256 byte values** | `cfg_22_find_needle_width_sweep` | [x] |
| 23 | `find_value_in_buffer` | random: 5 000 iterations of a random-content buffer (random length `0..=1024`) × random `i32` needle — finds first-occurrence semantics incl. duplicates | `cfg_23_find_random` | [x] |
| 24 | `find_value_in_buffer` | composed pipeline: buffer produced by `create_numeric_buffer` (C's own generator) then searched — the exact composition `doubleneg` performs, driven through the **low-level** exports | `cfg_24_find_over_generated_buffer` | [x] |
| 25 | `doubleneg` | `A11` = all 16 zero/non-zero combinations of the four params (using `0` and `1`) — stdout bytes + return value | `cfg_25_doubleneg_truthiness_16` | [x] |
| 26 | `doubleneg` | `A11` with *large* non-zero representatives (`0` vs `123456`) — 16 combinations | `cfg_26_doubleneg_truthiness_large` | [x] |
| 27 | `doubleneg` | `param1` swept `0..=256` (drives `create_numeric_buffer` seed ⇒ every possible buffer rotation, incl. the byte-100-absent case) with fixed other params | `cfg_27_doubleneg_seed_sweep` | [x] |
| 28 | `doubleneg` | `param2` swept over `{-300..=300 step 17}` (drives `search_values[0]`, the `i*param2` stride, and the `b` divisor incl. `b == 0`) | `cfg_28_doubleneg_param2_sweep` | [x] |
| 29 | `doubleneg` | `param3` swept over `{-25..=25}` (drives `c % 10` ⇒ every exponent, plus `search_values[1]`) | `cfg_29_doubleneg_param3_sweep` | [x] |
| 30 | `doubleneg` | `param4` swept over `{-300..=300 step 13}` (drives only `search_values[2]` and `!!param4`) | `cfg_30_doubleneg_param4_sweep` | [x] |
| 31 | `doubleneg` | extremes cross-product: each param ∈ {`INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX`} (2401 combinations) | `cfg_31_doubleneg_extremes_cross` | [x] |
| 32 | `doubleneg` | 2 000 fully random `(p1,p2,p3,p4)` over the whole `i32` range — stdout bytes + return value | `cfg_32_doubleneg_random` | [x] |
| 33 | *all six* | interleaved call sequence: the same pseudo-random script drives all six exports back-to-back on both libraries, so any hidden per-library state (buffering, statics) diverges | `cfg_33_interleaved_all_exports` | [x] |
| 34 | `create_numeric_buffer` + `find_value_in_buffer` | `A4`/`A6` = large sizes (`64 KiB`, `1 MiB`, `8 MiB`) × `A7` ∈ {`0`,`-1`,`12345`,`INT_MAX`,`INT_MIN`}, then searched at full length and at sub-ranges `{1,3,15,31,63,127,len/2,len-1}` — crosses glibc `memchr`'s SIMD block boundaries, which the small-buffer rows never reach | `cfg_34_large_buffers` | [x] |

All 33 rows have a passing differential test in `translation/tests/configs.rs`
(and `tests/doubleneg.rs` for the stdout-capturing rows 25–32).
