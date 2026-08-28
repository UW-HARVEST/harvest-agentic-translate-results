# CONFIGS.md — Phase A configuration-surface table

## How this table was derived

`c_src/include/lib.h` exposes one function, but the `.so` exports **12** (no
`static` functions in `lib.c`), so all 12 are public entry points and all 12 —
including the lowest-level ones (`increment_counter`, `update_accumulator`,
`add_three`, `multiply_add`, `complex_calc`, `apply_operation`,
`shift_array_data`, `process_pointer_data`) — are driven directly, not only
through the `hatch` convenience wrapper.

Axes the C code actually branches on / is sensitive to:

* **A. Hidden mutable configuration state.** `static int global_counter` and
  `static int global_accumulator` (lines 29–30) are library-global, never reset,
  and are *read* by `complex_calc` (line 56), `process_pointer_data` (line 75)
  and `hatch` (line 174), and *written* by `increment_counter` (line 36),
  `update_accumulator` (line 40) and — transitively — by every `hatch` call
  (lines 128–132). So the *same* call with the *same* arguments returns
  *different* values depending on history. States exercised:
  `fresh (0,0)` / `counter-only` / `accumulator-only` / `both, after N hatches` /
  `wrapped past INT_MAX`.
* **B. `shift_array_data` guard + shape** (line 67): `shift_by > 0 && shift_by < size`.
  Distinguished shapes: `size` ∈ {2, 3, 10, large}, `shift_by` ∈ {1, mid, size-1}.
* **C. `manipulate_records` guard + shape** (line 111): `shift > 0 && shift < num_records`,
  then a loop bound of `num_records - shift` (line 116). Distinguished shapes:
  `num_records` ∈ {1, 2, 5, many}, `shift` ∈ {1, mid, num_records-1}.
* **D. `compute_with_dynamic_memory` count shape** (lines 79–88): `count` ∈
  {1, 8 (the value `hatch` uses), many}; `base` sign/magnitude.
* **E. `apply_operation` callee selection** (line 44): the function pointer is
  the only "mode" flag in the API. Variants: `add_three`, `multiply_add`,
  `complex_calc` (the three `operation_func`s the library itself installs at
  lines 136/139/142), plus cross-library pointers.
* **F. Value shape of the `int` operands** everywhere: `0`, `±1`, small, large,
  `INT_MAX`, `INT_MIN`, mixed signs, and values that make the arithmetic wrap.
* **G. `hatch` argument interaction**: `param1..param4` feed 9 different
  sub-computations (lines 128–174); combinations of sign/magnitude across the
  four are distinct code paths through the composed pipeline.
* **H. Repetition / call-order**: `hatch` called 1×, 2×, N× in a row; and
  `hatch` interleaved with direct low-level calls that also touch the globals.
* **I. Build configuration**: `Cargo.toml` declares **no `[features]`**, so the
  only feature combinations are `default` (empty) and `--no-default-features`
  (also empty). `lib.c` has no `#ifdef`. Both are still run (see
  `run_all_features.sh`).

Every row is run against **many randomized inputs** (fixed seed, `xorshift64*`
PRNG, ≥ 200 iterations per row unless noted) and compared byte-for-byte between
the two `.so`s.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `add_three` | pure; randomized `(a,b,c)` over full `i32` range incl. extremes | `cfg_c1_add_three_random` | [x] |
| C2 | `multiply_add` | pure; randomized `(a,b,c)` full range (products wrap) | `cfg_c2_multiply_add_random` | [x] |
| C3 | `increment_counter` | state A: from a fresh `(0,0)` state, a long randomized sequence of `value`s; effect observed via `complex_calc(0,0,0)` after each step | `cfg_c3_increment_counter_sequence` | [x] |
| C4 | `update_accumulator` | state A: from fresh state, long randomized sequence; effect observed via `process_pointer_data(&0,0)` after each step | `cfg_c4_update_accumulator_sequence` | [x] |
| C5 | `complex_calc` | reads global state; randomized `(a,b,c)` × state ∈ {fresh, counter-only, both-set, wrapped} | `cfg_c5_complex_calc_vs_state` | [x] |
| C6 | `process_pointer_data` | reads global state; randomized `*ptr` × `multiplier` × state ∈ {fresh, accumulator-set, wrapped} | `cfg_c6_process_pointer_vs_state` | [x] |
| C7 | `process_pointer_data` | pointer shape: interior pointer into a heap array (`&arr[k]`, `k` randomized, mirrors `&dynamic_data[5]` at line 150) | `cfg_c7_process_pointer_interior` | [x] |
| C8 | `apply_operation` | mode E = `add_three`; randomized `(a,b,c)` | `cfg_c8_apply_op_add_three` | [x] |
| C9 | `apply_operation` | mode E = `multiply_add`; randomized `(a,b,c)` | `cfg_c9_apply_op_multiply_add` | [x] |
| C10 | `apply_operation` | mode E = `complex_calc` × state ∈ {fresh, dirty, wrapped}; randomized `(a,b,c)` | `cfg_c10_apply_op_complex_calc` | [x] |
| C11 | `apply_operation` | mode E = the *other* library's `add_three`/`multiply_add` (cross-`.so` function pointer) | `cfg_c11_apply_op_cross_library` | [x] |
| C12 | `shift_array_data` | guard true, `size = 10, shift_by = 3` (exactly the shape `hatch` uses, line 152); randomized array contents; full 40-byte buffer compared | `cfg_c12_shift_hatch_shape` | [x] |
| C13 | `shift_array_data` | guard true, `size = 2, shift_by = 1` (minimum accepted shape) | `cfg_c13_shift_min_shape` | [x] |
| C14 | `shift_array_data` | guard true, `shift_by = size - 1` (maximum accepted shift), `size` ∈ 2..64 randomized | `cfg_c14_shift_max_shift` | [x] |
| C15 | `shift_array_data` | guard true, randomized `size` ∈ 2..256 × randomized `shift_by` ∈ 1..size-1 × randomized contents; whole buffer + 16 bytes of trailing red-zone compared | `cfg_c15_shift_random_shapes` | [x] |
| C16 | `shift_array_data` | guard true, large shape `size = 4096`, `shift_by` random — exercises `memmove`'s large/overlapping path | `cfg_c16_shift_large` | [x] |
| C17 | `compute_with_dynamic_memory` | `count = 1` (single element) × randomized `base` | `cfg_c17_cwdm_count_one` | [x] |
| C18 | `compute_with_dynamic_memory` | `count = 8` (the value `hatch` uses, line 172) × randomized `base` incl. extremes | `cfg_c18_cwdm_count_eight` | [x] |
| C19 | `compute_with_dynamic_memory` | randomized `count` ∈ 1..1024 × randomized `base` | `cfg_c19_cwdm_random` | [x] |
| C20 | `get_time_based_value` | randomized `seed` in the non-overflowing band `|seed| <= 596523` (both signs, incl. 0 and ±1) | `cfg_c20_time_seed_band` | [x] |
| C21 | `get_time_based_value` | `seed` exactly at the overflow boundary 596523 / 596524 / −596523 / −596524 | `cfg_c21_time_seed_boundary` | [x] |
| C22 | `manipulate_records` | guard true, `num_records = 5, shift = 2` (exactly the shape `hatch` uses, line 168); randomized `value`s; return **and** the post-`memmove` 240-byte buffer image compared | `cfg_c22_records_hatch_shape` | [x] |
| C23 | `manipulate_records` | guard true, `num_records = 2, shift = 1` (minimum accepted shape) | `cfg_c23_records_min_shape` | [x] |
| C24 | `manipulate_records` | guard true, `shift = num_records - 1` (maximum accepted shift), `num_records` ∈ 2..32 | `cfg_c24_records_max_shift` | [x] |
| C25 | `manipulate_records` | guard true, randomized `num_records` ∈ 2..64 × `shift` ∈ 1..num_records-1 × randomized `id`/`value`/`timestamp`/`name` bytes; full struct image compared (pins the 48-byte layout and 0/4/8/16 offsets) | `cfg_c25_records_random_shapes` | [x] |
| C26 | `manipulate_records` | guard true, `value`s chosen so the running `total` wraps `int` | `cfg_c26_records_total_wrap` | [x] |
| C27 | `hatch` | fresh library state, single call, randomized `(p1..p4)` small magnitudes | `cfg_c27_hatch_small_fresh` | [x] |
| C28 | `hatch` | randomized `(p1..p4)` full `i32` range (every internal op wraps) | `cfg_c28_hatch_full_range` | [x] |
| C29 | `hatch` | `(p1..p4)` drawn from the extreme set {0, 1, −1, 2, −2, INT_MAX, INT_MIN, INT_MAX/2, INT_MIN/2} — full 9⁴ = 6561 cross-product | `cfg_c29_hatch_extreme_grid` | [x] |
| C30 | `hatch` | repetition axis H: 512 consecutive calls, comparing the result of **every** call (drives `global_counter`/`global_accumulator` through wrap-around, incl. the `*2` doubling overflow) | `cfg_c30_hatch_repeated` | [x] |
| C31 | `hatch` + low-level | interleaving axis H: randomized script mixing `hatch`, `increment_counter`, `update_accumulator`, `complex_calc`, `process_pointer_data`, `apply_operation` — the composed pipeline over shared state | `cfg_c31_interleaved_script` | [x] |
| C32 | all 12 entry points | one long randomized "fuzz script" (5000 steps) over every exported symbol with randomized shapes/values, comparing every observable (return values *and* mutated buffers) after every step | `cfg_c32_full_api_fuzz` | [x] |
| C33 | build config I | every feature combination (`default`, `--no-default-features`, `--all-features`) — `Cargo.toml` declares no features, and `lib.c` has no `#ifdef`, so all three are the same code path; verified by running the entire suite under each | `run_all_features.sh` | [x] |
| C34 | `compute_with_dynamic_memory` | large-but-reliable allocations: `count` ∈ {2^10, 2^14, 2^16, 2^18, 2^20} × `base` extremes — the `malloc`-succeeds path with a heavily wrapping `sum` | `err_e37_cwdm_large_but_valid_count` | [x] |
| C35 | build profile axis | the whole suite re-run against a **debug-profile** cdylib and a **release-profile** cdylib (the C reference is `-O0` and relies on signed-overflow wrap-around, so both Rust optimisation levels must agree with it). This axis is what uncovered the `debug-assertions` divergence recorded as E38 in `ERRORS.md`. | `run_all_features.sh` (section 4) | [x] |
| C36 | `increment_counter` / `update_accumulator` | the `unused_param` axis: the second argument is ignored by the C, so randomized junk is passed for it in every mutator call (`999`/`888` as `hatch` does, plus full-range random) | `cfg_c3_*`, `cfg_c4_*`, `cfg_c32_full_api_fuzz` | [x] |
