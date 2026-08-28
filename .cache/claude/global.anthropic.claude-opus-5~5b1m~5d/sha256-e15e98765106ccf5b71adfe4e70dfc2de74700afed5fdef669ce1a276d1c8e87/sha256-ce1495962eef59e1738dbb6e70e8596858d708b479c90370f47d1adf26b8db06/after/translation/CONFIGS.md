# CONFIGS.md — Phase B configuration-surface table

The mirror of `ERRORS.md`: every **valid** input configuration the C code
actually branches on. Axes derived mechanically from the `if` / `?:` / loop
conditions and from the data shapes `c_src/src/lib.c` special-cases. There are no
`#ifdef`s and no runtime option struct; the library's "options" are (a) which of
the 11 entry points is called, (b) the `operation_func` callback selection, and
(c) the `ResultArray.count` / index / floating-point input shapes.

## Axes the C branches on

| axis | values the C distinguishes |
|------|----------------------------|
| **A. entry point** | all 11 exported symbols — including the low-level ones (`init_result_array`, `process_with_foreach`, `compute_weighted_sum`, `compare_results_in_array`, `safe_double_to_int`), not just the `arrayfunc` one-shot wrapper |
| **B. `operation_func` selection** (`process_with_foreach`, line 126) | `add_operation`, `multiply_operation`, `subtract_operation`, `modulo_operation`, an *external* caller-supplied callback, NULL |
| **C. `ResultArray.count` shape** (lines 110, 112, 125, 140) | `0`, `1`, `2`, `3`, `9`, `10` (array capacity), `>10` (clamped by `init_result_array`), negative |
| **D. `count` provenance** | set by `init_result_array` (clamped) vs. hand-written into the struct field by the caller (unclamped) |
| **E. `int` value magnitude** | `0`, `±1`, small, near-`INT32_MAX`/`INT32_MIN`, exactly `INT32_MAX`/`INT32_MIN`, values that make `*0.75` / `*0.8` / `*1.5` / `*0.333` saturate |
| **F. `double` input class** (`safe_double_to_int`, lines 76–85) | `< INT32_MIN`, `== INT32_MIN`, in-range negative, `-0.0`, `+0.0`, subnormal, positive fraction `<1`, negative fraction (truncation **toward zero**), in-range positive, `== INT32_MAX`, `> INT32_MAX`, `±INFINITY`, NaN |
| **G. index pair** (`compare_results_in_array`, lines 94–106) | `idx1<idx2`, `idx1>idx2`, `idx1==idx2`, one/both `>= count`, negative, `count-1` boundary |
| **H. pipeline composition** (`arrayfunc`, lines 153–184) | one `process_with_foreach` pass vs. the full 4-op sequence; `compute_weighted_sum` on a **pristine** array vs. on an array already mutated by `process_with_foreach` (state carries over — `value` and `scaled` are rewritten in place) |
| **I. `arrayfunc` parameter shape** | all-zero, all-equal, small mixed sign, `param4` odd/even/negative (`/2` truncation), overflow-inducing `param3`, `INT32_MIN`/`INT32_MAX` extremes, random |

## Configuration rows

Each row is exercised with **many randomized inputs** (fixed seed, deterministic
xorshift PRNG) against both `.so`s, byte-for-byte.

| #  | entry point(s) | configuration (options set + input shape) | [x] | verifying test (`tests/phase_b_valid.rs`) |
|----|----------------|--------------------------------------------|-----|-------------------------------------------|
| C1  | `add_operation` | random `(a,b)` incl. `0`, `±1`, `INT32_MIN/MAX`, overflow-inducing pairs; `unused1/2` given garbage to prove they are ignored | [x] | `c1_add_operation` |
| C2  | `multiply_operation` | random `(a,b)` incl. overflow-inducing products, `0`, `±1`, extremes; garbage `unused1/2` | [x] | `c2_multiply_operation` |
| C3  | `subtract_operation` | random `(a,b)` incl. underflow-inducing pairs, extremes; garbage `unused1/2` | [x] | `c3_subtract_operation` |
| C4  | `modulo_operation` | `b != 0`, all four sign combinations of `(a,b)` (C truncates toward zero, so the remainder takes the **dividend's** sign); `b == ±1`; extremes | [x] | `c4_modulo_operation` |
| C5  | `safe_double_to_int` | axis **F**: every double class, plus randomized doubles spanning `±1e-300 … ±1e300` and random bit patterns reinterpreted as `double` | [x] | `c5_safe_double_to_int_all_classes` |
| C6  | `safe_double_to_int` | in-range values with fractional parts of both signs — verifies truncation toward zero, not floor | [x] | `c6_safe_double_to_int_truncation_toward_zero` |
| C7  | `compute_scaled_value` | `base` across axis **E** × `scale_factor` across axis **F** (cross-product incl. `0 * INF` → NaN, huge × huge → clamp) | [x] | `c7_compute_scaled_value` |
| C8  | `init_result_array` | `count` ∈ {0,1,2,3,9,10} with random `values[]`; asserts the **whole 248-byte struct** (`data[10]` incl. untouched tail + `count`) is byte-identical | [x] | `c8_init_result_array_normal_counts` |
| C9  | `init_result_array` | `count > 10` (11, 12, 100, `INT32_MAX`) → clamp to 10; full-struct byte compare | [x] | `c9_init_result_array_oversized_count_clamps` |
| C10 | `init_result_array` | called on a **pre-dirtied** struct (non-zero garbage in `data`/`count`) to confirm elements past `count` are left untouched identically | [x] | `c10_init_result_array_on_predirtied_struct` |
| C11 | `init_result_array` | `values[]` longer than `count` — proves only the first `count` entries are read | [x] | `c11_init_result_array_values_longer_than_count` |
| C12 | `process_with_foreach` | op = `add_operation` (from each library's **own** `.so`), `count` ∈ {0,1,2,3,9,10}; random seed values; compares return value **and** the mutated struct | [x] | `c12_process_with_foreach_add` |
| C13 | `process_with_foreach` | op = `multiply_operation`, same count sweep + random values | [x] | `c13_process_with_foreach_multiply` |
| C14 | `process_with_foreach` | op = `subtract_operation`, same count sweep + random values | [x] | `c14_process_with_foreach_subtract` |
| C15 | `process_with_foreach` | op = `modulo_operation`, same count sweep + random values (`rank` is the divisor, so `rank == 0` exercises the `b == 0` guard on element 0 of **every** run) | [x] | `c15_process_with_foreach_modulo` |
| C16 | `process_with_foreach` | op = **external Rust callback** passed into both libraries (the "arbitrary function pointer" axis) returning `INT32_MAX`, `INT32_MIN`, and value-dependent results | [x] | `c16_process_with_foreach_external_callback` |
| C17 | `process_with_foreach` | `count` hand-set (axis **D**) to a value ≤ 10 that differs from what `init_result_array` would have produced — proves `size` is read once from the struct field | [x] | `c17_process_with_foreach_hand_set_count` |
| C18 | `process_with_foreach` | applied **repeatedly** (2–5 passes) with mixed ops, so each pass sees the previous pass's rewritten `value`/`scaled` (axis **H** state carry-over) | [x] | `c18_process_with_foreach_repeated_passes` |
| C19 | `process_with_foreach` | element values chosen so `result * 0.75` lands exactly on `.5`/`.25`/`.75` boundaries and on the saturation thresholds | [x] | `c19_process_with_foreach_clamp_boundaries` |
| C20 | `compute_weighted_sum` | `count` ∈ {0,1,2,3,9,10} on a freshly `init_result_array`-initialised struct; random values | [x] | `c20_compute_weighted_sum_fresh` |
| C21 | `compute_weighted_sum` | on a struct already mutated by `process_with_foreach` (axis **H**) — the realistic `arrayfunc` ordering | [x] | `c21_compute_weighted_sum_after_process` |
| C22 | `compute_weighted_sum` | `value` fields hand-set to extremes so `value * weight * 0.8` saturates for some `i` but not others (weight varies with `i`, so the same value clamps at high `i` only) | [x] | `c22_compute_weighted_sum_saturating_values` |
| C23 | `compute_weighted_sum` | `count == 1` — exercises **only** the `weight = 1` fallback branch (axis **E28**) with no `weight = i` iteration | [x] | `c23_compute_weighted_sum_count_one_weight_fallback` |
| C24 | `compare_results_in_array` | all `(idx1, idx2)` pairs in `-3..=12` × `-3..=12` for every `count` in `-1..=11` (full cross-product, 3 549 cases) | [x] | `c24_compare_results_in_array_full_grid` |
| C25 | `compare_results_in_array` | randomized `(count, idx1, idx2)` over the full `int` range near the boundaries | [x] | `c25_compare_results_in_array_randomized` |
| C26 | full low-level pipeline | `init_result_array` → 4× `process_with_foreach` (add, multiply, subtract, modulo) → `compute_weighted_sum` → `compare_results_in_array` sweep — i.e. `arrayfunc` reassembled from the low-level exports, comparing the struct after **every** step | [x] | `c26_full_low_level_pipeline` |
| C27 | `arrayfunc` | axis **I**: hand-picked shapes — all-zero, all-one, all-equal, mixed sign, `param4` odd/even/negative, `param3` overflow, `INT32_MIN`/`INT32_MAX` in each of the 4 positions | [x] | `c27_arrayfunc_handpicked_shapes` |
| C28 | `arrayfunc` | 20 000 randomized `(param1..param4)` quadruples, small-magnitude (`-1000..1000`) — dense coverage of ordinary values | [x] | `c28_arrayfunc_random_small` |
| C29 | `arrayfunc` | 20 000 randomized quadruples over the **full** `int32` range — exercises the wraparound and saturation paths | [x] | `c29_arrayfunc_random_full_range` |
| C30 | `arrayfunc` | exhaustive sweep of the 4 params over `{-2,-1,0,1,2}^4` (625 cases) — boundary-dense small values incl. `param4/2` truncation for negatives | [x] | `c30_arrayfunc_exhaustive_small_grid` |
| C31 | `process_with_foreach` | `count` hand-set **past** the 10-element capacity (11, 12, 16, 64, 100, 999, 4096) over a 5 000-element mapped backing buffer, for all four ops — the marching-past-the-array configuration, made deterministic. Note the loop overwrites its own `count` field (offset 240 == `data[10].value`) as it goes, and `size` is read only once | [x] | `phase_c_errors::e24_deterministic_out_of_bounds_marching_process` |
| C32 | `compute_weighted_sum` | same oversized `count` on a large mapped buffer — the path where `weight` keeps growing as `i` past index 10 | [x] | `phase_c_errors::e24_deterministic_out_of_bounds_marching_weighted_sum` |
| C33 | `init_result_array`, `compare_results_in_array` | oversized `count` on a large mapped buffer: `init` must still clamp to 10 and write nothing past element 10; `compare` must do pure address arithmetic for far indices (up to 4095) | [x] | `phase_c_errors::e24_deterministic_out_of_bounds_marching_init_and_compare` |

## Feature combinations

`Cargo.toml` has no `[features]` table, so the complete feature space is the
single default (empty) combination. `check_features.sh` enumerates it
programmatically and re-runs the whole suite for it, so the "every combination"
requirement is met by construction rather than by assumption.

## Result

**33 / 33 rows pass**, across randomized inputs, in **both** the debug and the
release profile, and under every feature selection enumerated by
`./check_features.sh` (`default`, `--no-default-features`, `--all-features` — all
identical, since the crate declares no features). `./mutation_check.sh` confirms
these rows would actually catch a regression: 40 injected bugs were all detected,
and the 5 provably behaviour-preserving changes all survived as expected.
