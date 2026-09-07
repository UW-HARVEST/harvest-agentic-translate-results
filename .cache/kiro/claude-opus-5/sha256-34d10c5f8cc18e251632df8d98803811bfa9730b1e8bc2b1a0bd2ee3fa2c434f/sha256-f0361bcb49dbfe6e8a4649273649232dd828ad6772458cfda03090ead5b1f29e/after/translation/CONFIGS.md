# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`. Every row
is a combination of the axes the C code actually branches on, and every row is
driven through **both** `.so` files via `libloading` with **many randomized
inputs** (xorshift64\* PRNG, fixed seed `0x9E3779B97F4A7C15`) unless the row is
an exhaustive enumeration.

## Axes the C branches on

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| **Entry point** | all 11 exported functions — the 4 `operation_func` leaves, the 2 numeric helpers, the 4 `ResultArray` low-level functions, and the single header-declared wrapper `arrayfunc` | whole file |
| **`operation_func op`** (the only runtime "mode" the API accepts) | `add_operation`, `multiply_operation`, `subtract_operation`, `modulo_operation` | `process_with_foreach` param; `arrayfunc` iterates all 4 in that fixed order |
| **`arr->count`** | `0`, `1`, `2`, mid, `9`, `10` (max in-bounds), `>10` (clamped by `init_result_array`), `<0` | line 110 ternary; loop bounds at 112, 125, 140, 176 |
| **`double` class** | `>= (double)INT32_MAX`, `<= (double)INT32_MIN`, NaN, in-range fractional, in-range exact integer, `±0.0`, subnormal, `±Inf` | `safe_double_to_int` lines 76–85 |
| **`int` value class** | `0`, `±1`, small, near `INT32_MAX`, near `INT32_MIN` (drives overflow in the ops and saturation in `safe_double_to_int`) | lines 53, 59, 65, 72, 89, 115, 129, 146, 162–163, 181 |
| **index ordering** in `compare_results_in_array` | `idx1 < idx2`, `idx1 == idx2`, `idx1 > idx2`, `idx >= count`, `idx < 0` | lines 94, 101, 103 |
| **`current > base`** in `compute_weighted_sum` | `i == 0` → weight forced to `1`; `i > 0` → weight `= i` | line 144 |
| **scale constants** | `1.5` (init), `0.75` (foreach), `0.8` (weighted sum), `0.333` (final), plus arbitrary caller-supplied scale in `compute_scaled_value` | lines 115, 129, 146, 181, 89 |
| **array state carry-over** | `process_with_foreach` **mutates** `value` and `scaled` in place, so each successive call sees different data — output is order- and history-dependent | lines 130–131, called 4× from 171 |
| **build config** | none — `Cargo.toml` has no `[features]`; no `#ifdef` in the C | — |

## Table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `add_operation` | 4096 random full-range `(a,b)` + random non-zero `unused1/unused2` | `cfg_add_random` | [x] |
| 2 | `add_operation` | exhaustive cross product of `{0,±1,±2,INT32_MAX,INT32_MIN,INT32_MAX-1,INT32_MIN+1}` | `cfg_add_boundaries` | [x] |
| 3 | `multiply_operation` | 4096 random full-range `(a,b)` | `cfg_mul_random` | [x] |
| 4 | `multiply_operation` | exhaustive boundary cross product (overflowing products) | `cfg_mul_boundaries` | [x] |
| 5 | `subtract_operation` | 4096 random full-range `(a,b)` | `cfg_sub_random` | [x] |
| 6 | `subtract_operation` | exhaustive boundary cross product | `cfg_sub_boundaries` | [x] |
| 7 | `modulo_operation` | 4096 random `a`, random non-zero `b` of both signs | `cfg_mod_random` | [x] |
| 8 | `modulo_operation` | `b ∈ 0..=9` (the `rank` divisors `arrayfunc` actually produces) × random full-range `a` | `cfg_mod_rank_divisors` | [x] |
| 9 | `modulo_operation` | boundary `a` × `b ∈ {±1,±2,±3,INT32_MAX,INT32_MIN}`, excluding the SIGFPE pair (ERRORS #2) | `cfg_mod_boundaries` | [x] |
| 10 | all 4 ops | `unused1`/`unused2` set to random garbage — must be ignored identically | `cfg_ops_unused_args_ignored` | [x] |
| 11 | `safe_double_to_int` | in-range fractional values, positive and negative (truncate-toward-zero) | `cfg_sdti_fractional` | [x] |
| 12 | `safe_double_to_int` | in-range exact-integer doubles sampled across the full `int` range | `cfg_sdti_exact_integers` | [x] |
| 13 | `safe_double_to_int` | boundary neighbourhood: `±2147483645/6/7/8/9` and `next_after` steps either side of both clamps | `cfg_sdti_boundary_neighbourhood` | [x] |
| 14 | `safe_double_to_int` | specials: `+0.0`, `-0.0`, subnormal, `DBL_MIN`, `DBL_MAX`, `±Inf`, quiet/signalling/negative NaN | `cfg_sdti_specials` | [x] |
| 15 | `safe_double_to_int` | 8192 random 64-bit patterns reinterpreted as `f64` (hits every FP class, incl. NaN payloads) | `cfg_sdti_random_bitpatterns` | [x] |
| 16 | `compute_scaled_value` | random full-range `base` × fractional scale in `(-1,1)` | `cfg_csv_fractional_scale` | [x] |
| 17 | `compute_scaled_value` | random `base` × the library's own constants `{1.5, 0.75, 0.8, 0.333}` and their negations | `cfg_csv_library_constants` | [x] |
| 18 | `compute_scaled_value` | random `base` × random 64-bit-pattern scale (Inf / NaN / huge / subnormal) | `cfg_csv_random_scale` | [x] |
| 19 | `compute_scaled_value` | boundary `base ∈ {0,±1,INT32_MAX,INT32_MIN}` × boundary scale `{0.0,-0.0,1.0,-1.0,2.0,±Inf,NaN}` | `cfg_csv_boundaries` | [x] |
| 20 | `compare_results_in_array` | `count ∈ 1..=10` × **all** in-range `(idx1,idx2)` pairs (covers `<`, `==`, `>`) | `cfg_compare_all_inrange_pairs` | [x] |
| 21 | `init_result_array` | `count ∈ {0,1,2,5,8,9,10}` × random values; full struct state compared field-by-field | `cfg_init_counts_random` | [x] |
| 22 | `init_result_array` | `count = 8` (what `arrayfunc` uses) with extreme values `INT32_MAX`/`INT32_MIN` — checks `(double)v * 1.5` | `cfg_init_extreme_values` | [x] |
| 23 | `init_result_array` | `count ∈ {11,12,100,INT32_MAX}` → clamped to 10 | `cfg_init_clamped` | [x] |
| 24 | `init_result_array` | called twice on the same buffer, second time with a smaller `count` → stale elements must survive identically | `cfg_init_reinit_stale` | [x] |
| 25 | `process_with_foreach` | `op = add_operation`, `count ∈ 0..=10`, random values; return **and** mutated array compared | `cfg_foreach_add` | [x] |
| 26 | `process_with_foreach` | `op = multiply_operation`, `count ∈ 0..=10`, random values | `cfg_foreach_multiply` | [x] |
| 27 | `process_with_foreach` | `op = subtract_operation`, `count ∈ 0..=10`, random values | `cfg_foreach_subtract` | [x] |
| 28 | `process_with_foreach` | `op = modulo_operation`, `count ∈ 0..=10`, random values | `cfg_foreach_modulo` | [x] |
| 29 | `process_with_foreach` | each op × values near `INT32_MAX`/`INT32_MIN` so `result * 0.75` saturates in `safe_double_to_int` | `cfg_foreach_saturating` | [x] |
| 30 | `process_with_foreach` | same op applied 3× in a row — exercises the value-dependent state carry-over | `cfg_foreach_repeated` | [x] |
| 31 | `process_with_foreach` | all 4 ops in `arrayfunc`'s order on a carried-over array, `count ∈ 0..=10` | `cfg_foreach_all_ops_sequence` | [x] |
| 32 | `process_with_foreach` | **cross-ABI**: the C `.so`'s `op` function pointers fed into the Rust `process_with_foreach`, and the Rust `.so`'s into the C one | `cfg_foreach_cross_abi` | [x] |
| 33 | `compute_weighted_sum` | `count ∈ 0..=10` × random values | `cfg_weighted_counts_random` | [x] |
| 34 | `compute_weighted_sum` | `count = 10` with values at `±INT32_MAX` → per-term clamp + wrapping `sum` | `cfg_weighted_saturating` | [x] |
| 35 | `compute_weighted_sum` | `count = 1` — `current == base`, so the `? :` takes the `weight = 1` branch | `cfg_weighted_single` | [x] |
| 36 | low-level pipeline | `init(8)` → `foreach`×4 (add,mul,sub,mod) → `weighted_sum` → `compare` chain: `arrayfunc` rebuilt from its parts, **every intermediate** compared | `cfg_pipeline_arrayfunc_replica` | [x] |
| 37 | low-level pipeline | the same pipeline for `count ∈ 0..=10`, not just `arrayfunc`'s 8 | `cfg_pipeline_all_counts` | [x] |
| 38 | low-level pipeline | the same pipeline with all **24 permutations** of the 4 ops (order matters because of the in-place mutation) | `cfg_pipeline_op_permutations` | [x] |
| 39 | `arrayfunc` | 8192 random full-range `(p1,p2,p3,p4)` | `cfg_arrayfunc_random` | [x] |
| 40 | `arrayfunc` | dense small grid `p ∈ -6..=6` (hits `rank`-vs-`value` interactions and `param4/2` rounding for both signs) | `cfg_arrayfunc_small_grid` | [x] |
| 41 | `arrayfunc` | exhaustive cross product of `{0,±1,±2,INT32_MAX,INT32_MIN,INT32_MAX/2,INT32_MIN/2,1<<30,-(1<<30)}` — overflow in `p1+p2`, `p2-p3`, `p3*2`, and the `p4/2` boundary | `cfg_arrayfunc_boundaries` | [x] |
| 42 | `arrayfunc` | one param extreme, the other three random (isolates each derived slot) | `cfg_arrayfunc_one_extreme` | [x] |
| 43 | `ResultArray` ABI | struct layout round-trip: `init_result_array` from one `.so`, then `compute_weighted_sum` / `compare_results_in_array` / `process_with_foreach` from the **other**, plus asserted field offsets (0/8/16, `count` @240, size 248) | `cfg_layout_roundtrip` | [x] |

## Feature combinations

`translation/Cargo.toml` declares no `[features]`, so there is exactly one
configuration. `run_all.sh` still runs the suite under
`--no-default-features` and with `--all-features` to prove equivalence, and
additionally re-runs the whole suite against the **debug** Rust `.so`
(`overflow-checks = on`) as well as the **release** one, since a translation
that relied on implicit wrapping would panic in the former.
