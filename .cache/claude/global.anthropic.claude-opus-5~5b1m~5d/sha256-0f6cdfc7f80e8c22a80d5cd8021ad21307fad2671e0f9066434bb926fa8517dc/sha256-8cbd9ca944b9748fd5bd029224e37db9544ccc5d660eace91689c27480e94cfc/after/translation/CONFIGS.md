# CONFIGS.md — Phase B valid-path configuration surface table

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## The axes the C branches on

This library has **no runtime option/flag setters and no `#ifdef`**. Its
"configuration" is entirely (a) *which entry point* is called, (b) *the shape
of the integer/string argument*, and (c) **the hidden `static` state left over
from previous calls** — which is the real configuration axis, since three
`static` variables persist for the lifetime of the loaded `.so`.

**Axis 1 — entry point** (all 8 exported symbols, including the four
lowest-level `operation_func` targets that `findrep` dispatches through; these
are tested *directly*, not only via the `findrep` wrapper):
`add_to_accumulator`, `multiply_with_multiplier`, `subtract_from_accumulator`,
`divide_multiplier`, `process_octal_string`, `find_and_replace_char`,
`validate_and_normalize`, `findrep`.

**Axis 2 — `validate_and_normalize` value bucket** (`lib.c:81-87`), which is
applied to each of `findrep`'s four params:
`Z` = 0 · `N` = negative · `L` = `0 < v < 0100` (lower-clamped to 64) ·
`P` = `0100 <= v <= 0777` (passthrough) · `U` = `v > 0777` (clamped to 511).

**Axis 3 — `active_params` bucket** (`lib.c:107`, compared against `mode_add=01`
and `mode_multiply=02`; `mode_subtract=03`/`mode_divide=04` are dead constants):
`0` (no dispatch) · `1` (add only) · `2..4` (add + multiply).

**Axis 4 — hidden-state guards**: `accumulator > 0150` (`lib.c:142`),
`multiplier > 0100` (`lib.c:161`), `both_active = accumulator && multiplier`
(`lib.c:155`), and the `result == 0` sentinel (`lib.c:169`).

**Axis 5 — call-sequence length**: fresh library (`accumulator=0, multiplier=1,
operation_count=0`) vs. state accumulated over 2, 3, … N prior calls. Every
`operation_func` mutates state, so the Nth call's return value depends on all
N-1 before it. Randomized long sequences are the only way to cover this.

**Axis 6 — string shape** for `find_and_replace_char` / `process_octal_string`:
empty · 1 byte · many bytes · match at index 0 / middle / last · no match ·
repeated matches · high-bit (non-ASCII) bytes · byte values needing
`unsigned char` narrowing.

## Configuration rows

Each row is checked off only after passing **many randomized inputs** with a
fixed seed (deterministic xorshift64*, `harness::Rng` in `tests/harness/mod.rs`).
Row tests live in `tests/phase_b_valid.rs`.

| #  | entry point(s) | configuration (options set + input shape) | test | [ ] |
|----|----------------|--------------------------------------------|------|-----|
| 1  | `validate_and_normalize` | bucket `Z`: `v == 0` | `cfg_01_normalize_bucket_z` | [x] |
| 2  | `validate_and_normalize` | bucket `N`: randomized `v < 0` incl. `INT_MIN`, `-1` | `cfg_02_normalize_bucket_n` | [x] |
| 3  | `validate_and_normalize` | bucket `L`: randomized `1 <= v <= 63` (exhaustive) | `cfg_03_normalize_bucket_l` | [x] |
| 4  | `validate_and_normalize` | bucket `P`: exhaustive `64 <= v <= 511` | `cfg_04_normalize_bucket_p` | [x] |
| 5  | `validate_and_normalize` | bucket `U`: randomized `v >= 512` incl. `INT_MAX` | `cfg_05_normalize_bucket_u` | [x] |
| 6  | `validate_and_normalize` | full-range randomized sweep across all buckets | `cfg_06_normalize_random_sweep` | [x] |
| 7  | `add_to_accumulator` | fresh state, one call, randomized `(a,b)` full `i32` range | `cfg_07_add_single_fresh` | [x] |
| 8  | `add_to_accumulator` | long randomized call sequence — accumulates state, drives `accumulator` through both signs and past `0150` | `cfg_08_add_sequence` | [x] |
| 9  | `subtract_from_accumulator` | fresh state, one call, randomized `(a,b)` | `cfg_09_sub_single_fresh` | [x] |
| 10 | `subtract_from_accumulator` | long randomized sequence (state-dependent) | `cfg_10_sub_sequence` | [x] |
| 11 | `add_to_accumulator` + `subtract_from_accumulator` | interleaved randomized sequence — both mutate the *same* static | `cfg_11_add_sub_interleaved` | [x] |
| 12 | `multiply_with_multiplier` | fresh state, one call, randomized `(a,b)` — includes wrapping products | `cfg_12_mul_single_fresh` | [x] |
| 13 | `multiply_with_multiplier` | long randomized sequence — `multiplier` saturates to 0 or wraps chaotically | `cfg_13_mul_sequence` | [x] |
| 14 | `multiply_with_multiplier` | operands constrained small (`-3..3`) so `multiplier` stays meaningful and crosses `0100` in both directions | `cfg_14_mul_small_operands` | [x] |
| 15 | `divide_multiplier` | fresh state, randomized `b != 0` (`a` is ignored by the C) | `cfg_15_div_single_fresh` | [x] |
| 16 | `divide_multiplier` | long randomized sequence with `b != 0`, positive and negative divisors | `cfg_16_div_sequence` | [x] |
| 17 | `multiply_with_multiplier` + `divide_multiplier` | interleaved randomized sequence — both mutate `multiplier`; checks truncation-toward-zero composes identically | `cfg_17_mul_div_interleaved` | [x] |
| 18 | all four `operation_func` targets | fully randomized interleaving of all 4, long sequence — the exact composed pipeline `findrep` builds, driven at the lowest level | `cfg_18_all_ops_interleaved` | [x] |
| 19 | `process_octal_string` | `octal_val == 0`, `1`, `7`, `8`, `0123` (the literal `findrep` uses), `0777` | `cfg_19_octal_small_values` | [x] |
| 20 | `process_octal_string` | randomized positive `octal_val` over full `i32` range | `cfg_20_octal_random_positive` | [x] |
| 21 | `process_octal_string` | randomized negative `octal_val` — `%o` unsigned vs `%d` signed | `cfg_21_octal_random_negative` | [x] |
| 22 | `process_octal_string` | extremes `INT_MAX`, `INT_MIN`, `-1`, and each power-of-two boundary where the octal digit count changes | `cfg_22_octal_length_boundaries` | [x] |
| 23 | `find_and_replace_char` | match at index 0 | `cfg_23_replace_at_start` | [x] |
| 24 | `find_and_replace_char` | match at the last byte before the NUL | `cfg_24_replace_at_end` | [x] |
| 25 | `find_and_replace_char` | match in the middle; also the `'O'` / `"Octal: ..."` pairing `findrep` uses | `cfg_25_replace_middle_and_findrep_case` | [x] |
| 26 | `find_and_replace_char` | 1-byte string, matching and non-matching | `cfg_26_replace_single_byte` | [x] |
| 27 | `find_and_replace_char` | randomized byte strings × randomized `search_char` over the full `i32` range (covers narrowing, high-bit bytes, no-match) — whole buffer compared incl. bytes past the NUL | `cfg_27_replace_randomized` | [x] |
| 28 | `find_and_replace_char` | strings containing embedded high-bit bytes `0x80..0xFF`, searched for both as positive and as negative `int` | `cfg_28_replace_high_bit_bytes` | [x] |
| 29 | `findrep` | `active_params == 0`: `(0,0,0,0)` on a fresh library | `cfg_29_findrep_active_0` | [x] |
| 30 | `findrep` | `active_params == 1`: each of the 4 single-nonzero positions, fresh library each time | `cfg_30_findrep_active_1_each_position` | [x] |
| 31 | `findrep` | `active_params == 2`: all 6 position pairs | `cfg_31_findrep_active_2_all_pairs` | [x] |
| 32 | `findrep` | `active_params == 3`: all 4 position triples | `cfg_32_findrep_active_3_all_triples` | [x] |
| 33 | `findrep` | `active_params == 4`: all params nonzero | `cfg_33_findrep_active_4` | [x] |
| 34 | `findrep` | full cross-product of the 5 value buckets `{Z,N,L,P,U}` over 4 params = 625 bucket combinations, one randomized representative each | `cfg_34_findrep_bucket_cross_product` | [x] |
| 35 | `findrep` | `accumulator > 0150` guard TRUE — params chosen so the add pushes `accumulator` past 104 | `cfg_35_findrep_accumulator_guard_true` | [x] |
| 36 | `findrep` | `multiplier > 0100` guard TRUE — reached after a multiply that leaves `multiplier > 64` | `cfg_36_findrep_multiplier_guard_true` | [x] |
| 37 | `findrep` | `both_active` TRUE (both statics nonzero) vs FALSE | `cfg_37_findrep_both_active` | [x] |
| 38 | `findrep` | repeated identical calls on the SAME loaded library — return value must drift identically as state accumulates (10 iterations) | `cfg_38_findrep_repeated_same_args` | [x] |
| 39 | `findrep` | long randomized call sequence (200 calls, full `i32` params) on one loaded library — the deepest state-dependence test | `cfg_39_findrep_long_random_sequence` | [x] |
| 40 | `findrep` | randomized params restricted to small values `-4..4`, long sequence — keeps `multiplier` from latching to 0 so the divide branch stays live | `cfg_40_findrep_small_params_sequence` | [x] |
| 41 | `findrep` + all `operation_func` targets | randomized sequence mixing `findrep` calls with direct low-level operation calls, so `findrep` observes state it did not create itself | `cfg_41_mixed_findrep_and_direct_ops` | [x] |
| 42 | `findrep` + `validate_and_normalize` + `process_octal_string` + `find_and_replace_char` | randomized sequence mixing the stateless helpers in between stateful calls, confirming they do not perturb the statics | `cfg_42_stateless_helpers_do_not_perturb_state` | [x] |
| 43 | `findrep` | params at exact bucket boundaries `{0, 1, 63, 64, 511, 512, INT_MIN, INT_MAX}` cross-product sample | `cfg_43_findrep_boundary_params` | [x] |
| 44 | `findrep` | `INT_MIN`/`INT_MAX` params driving `accumulator`/`multiplier` overflow inside the dispatched ops | `cfg_44_findrep_overflow_params` | [x] |

## Verification results

All 44 rows pass across randomized inputs, in **every** build configuration.
This crate declares no `[features]` and no `[[bin]]`/`src/bin` target, so the
configuration set is:

| profile | feature combo | phase B | stress | phase C | phase D |
|---------|---------------|---------|--------|---------|---------|
| debug   | default (`--features=`)  | 44/44 | 6/6 | 36/36 | 3/3 |
| debug   | `--no-default-features`   | 44/44 | 6/6 | 36/36 | 3/3 |
| release | default (`--features=`)  | 44/44 | 6/6 | 36/36 | 3/3 |
| release | `--no-default-features`   | 44/44 | 6/6 | 36/36 | 3/3 |

Both profiles are exercised because the Rust `.so` genuinely differs between
them: `release` sets `panic = "abort"`, and `debug` enables integer-overflow
checks (which pass only because every arithmetic op in the translation uses the
`wrapping_*` family, matching the C's two's-complement wraparound).

Reproduce with `bash .scratch/all_configs.sh`.

There is **no driver binary**, so the "compare C and Rust stdout" gate does not
apply; the C `CMakeLists.txt` builds only `add_library(... SHARED src/lib.c)`.

## Beyond the rows

`tests/phase_bc_stress.rs` adds 6 configuration-independent probes with
different seeds and exhaustive grids, so a divergence that the per-row inputs
happen to miss still surfaces:

* `validate_and_normalize` exhaustive over `-2048..=2048` plus every
  power-of-two neighbourhood across the full `i32` range;
* `process_octal_string` exhaustive over `-4096..=4096`, every power-of-two
  neighbourhood, and 20 000 random values, comparing the full 64-byte buffer;
* `find_and_replace_char` over the complete 255 × 261 (byte × search_char)
  matrix including negative and `>255` search chars;
* `findrep` exhaustive over the dense `[-2,2]^4` grid on fresh state;
* six independent 3 000-call mixed sequences over all eight entry points;
* three independent 2 000-call bucket-combination sequences in differing orders.
