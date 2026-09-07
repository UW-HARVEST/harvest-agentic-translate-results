# CONFIGS.md — configuration-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`: the axes
below are exactly the things the C code `if`s / `switch`es on. There are no
`#ifdef`s, no global/runtime configuration state, no environment variables and
no `[features]` in `translation/Cargo.toml`, so the only configuration is the
argument tuple passed across the FFI boundary.

## Axes the C actually branches on

* **Entry point** (all four exported symbols, including the three low-level
  operation callbacks that `gotomach` merely composes):
  `gotomach`, `process_value`, `double_value`, `triple_value`.
* **`mode`** — `switch (mode)`: `0` ⇒ `process_value`, `1` ⇒ `double_value`,
  `2` ⇒ `triple_value`, `default` ⇒ warning + `process_value`. Four distinct
  paths; the `default` arm is reachable from below (`< 0`) and above (`> 2`).
* **`iterations`** — shape/count axis. Special-cased values: `0` (loop never
  runs, and `malloc(0)`), `1` (single iteration), "many", and the upper bound
  `65535` (also the only value that can make the line-178 max-count `break`
  fire).
* **`seed`** — the initial `current_value`, range-checked to `[0, 65535]`.
  Special values `0`, `1`, `65535`; the value matters because it feeds a
  data-dependent chain (`current_value = produced % 1000`).
* **`threshold`** — selects, per iteration, whether the produced value is
  stored (`produced < threshold`, strict). Distinct regimes: below every
  produced value (nothing stored ⇒ result 0), above every produced value
  (everything stored), straddling (some stored — the interesting mixed case),
  and exactly equal to a produced value (boundary of the strict `<`).
* **Callback argument shape** — `unused_param` (any `int`) and
  `unused_context` (`NULL` vs non-`NULL`); the C ignores both, so both must be
  ignored identically. Plus the `value` axis: `0`, positive, negative, and the
  signed-overflow extremes `INT_MIN` / `INT_MAX`.

## Rows

Each row is exercised with **many randomized inputs from a fixed seed**
(deterministic xorshift PRNG in the test), and both the return value **and the
byte-exact stdout** of the C and Rust `.so`s are compared.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `process_value` | `value` random over the full `int` range; `unused_param` random; `unused_context` `NULL` | [x] |
| 2 | `process_value` | `value` ∈ {0, 1, -1, 10, -10, `INT_MAX`, `INT_MAX-9`, `INT_MIN`} (overflow boundaries of `value + 10`); `unused_context` non-`NULL` garbage | [x] |
| 3 | `double_value` | `value` random over the full `int` range; `unused_param` random; `unused_context` `NULL` | [x] |
| 4 | `double_value` | `value` ∈ {0, 1, -1, `INT_MAX`, `INT_MAX/2`, `INT_MIN`, `INT_MIN/2`} (overflow boundaries of `value * 2`); `unused_context` non-`NULL` garbage | [x] |
| 5 | `triple_value` | `value` random over the full `int` range; `unused_param` random; `unused_context` `NULL` | [x] |
| 6 | `triple_value` | `value` ∈ {0, 1, -1, `INT_MAX`, `INT_MAX/3`, `INT_MIN`, `INT_MIN/3`} (overflow boundaries of `value * 3`); `unused_context` non-`NULL` garbage | [x] |
| 7 | `gotomach` | `mode = 0`, `iterations = 0` (loop body never executes, `malloc(0)`), `seed`/`threshold` randomized | [x] |
| 8 | `gotomach` | `mode = 0`, `iterations = 1`, `seed`/`threshold` randomized | [x] |
| 9 | `gotomach` | `mode = 0`, `iterations` random in `[2, 4096]`, `threshold` in the *straddling* regime (some values stored, some not), `seed` randomized | [x] |
| 10 | `gotomach` | `mode = 0`, `iterations` random, `threshold = INT_MIN` (nothing ever stored, `count` stays 0, sum is 0) | [x] |
| 11 | `gotomach` | `mode = 0`, `iterations` random, `threshold = INT_MAX` (every produced value stored, `count == iterations`) | [x] |
| 12 | `gotomach` | `mode = 1` (`double_value`), `iterations = 0` | [x] |
| 13 | `gotomach` | `mode = 1`, `iterations = 1` | [x] |
| 14 | `gotomach` | `mode = 1`, `iterations` random in `[2, 4096]`, straddling `threshold`, randomized `seed` | [x] |
| 15 | `gotomach` | `mode = 1`, `iterations` random, `threshold = INT_MIN` | [x] |
| 16 | `gotomach` | `mode = 1`, `iterations` random, `threshold = INT_MAX` | [x] |
| 17 | `gotomach` | `mode = 2` (`triple_value`), `iterations = 0` | [x] |
| 18 | `gotomach` | `mode = 2`, `iterations = 1` | [x] |
| 19 | `gotomach` | `mode = 2`, `iterations` random in `[2, 4096]`, straddling `threshold`, randomized `seed` | [x] |
| 20 | `gotomach` | `mode = 2`, `iterations` random, `threshold = INT_MIN` | [x] |
| 21 | `gotomach` | `mode = 2`, `iterations` random, `threshold = INT_MAX` | [x] |
| 22 | `gotomach` | `mode` in the `default` arm from above (`mode` random in `[3, INT_MAX]`), full randomized `iterations`/`seed`/`threshold` — must log the warning and behave as `mode = 0` | [x] |
| 23 | `gotomach` | `mode` in the `default` arm from below (`mode` random in `[INT_MIN, -1]`), full randomized `iterations`/`seed`/`threshold` | [x] |
| 24 | `gotomach` | `seed` boundary values `{0, 1, 65535}` × `mode` `{0,1,2,default}`, `iterations` small, `threshold` randomized | [x] |
| 25 | `gotomach` | `threshold` set *exactly* to a produced value (strict-`<` boundary): `mode` 0/1/2 with `iterations = 1` and `threshold ∈ {f(seed), f(seed)+1, f(seed)-1}` | [x] |
| 26 | `gotomach` | `iterations = 65535` (upper bound) × `mode` `{0,1,2}` × `threshold ∈ {INT_MIN, INT_MAX, straddling}` — the only shape that can reach the line-178 `[WARNING] Reached maximum count` `break` | [x] |
| 27 | `gotomach` | fully unconstrained random fuzz: all four `int` parameters drawn from the full `int` range (mixes valid and invalid, exercises the interaction of the range checks with the `switch`) | [x] |
| 28 | `gotomach` | repeated back-to-back invocations in one process (state must not leak between calls: `count`/`capacity` are per-call heap state) | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]`** table and no optional
dependencies, so the complete set of feature combinations is the single empty
combination. `cargo test --no-default-features` is therefore identical to
`cargo test`; both are run by `run_all.sh` for completeness. There is likewise
**no `[[bin]]`/binary driver target** in the crate and no `add_executable` in
`c_src/CMakeLists.txt`, so the "compare binary stdout" gate is satisfied by the
per-call stdout capture used in every row above.

## Row → test mapping and results

Every row is checked off above only because the named test passed against both
`.so`s. Each `diff_gotomach` / `diff_op` call compares **the return value and
the byte-exact stdout** of the two libraries.

| rows | test file | test name(s) |
|------|-----------|--------------|
| 1 | `tests/configs.rs` | `row01_process_value_random_null_ctx` |
| 2 | `tests/configs.rs` | `row02_process_value_overflow_boundaries_garbage_ctx` |
| 3 | `tests/configs.rs` | `row03_double_value_random_null_ctx` |
| 4 | `tests/configs.rs` | `row04_double_value_overflow_boundaries_garbage_ctx` |
| 5 | `tests/configs.rs` | `row05_triple_value_random_null_ctx` |
| 6 | `tests/configs.rs` | `row06_triple_value_overflow_boundaries_garbage_ctx` |
| 7–11 | `tests/configs.rs` | `row07_mode0_zero_iterations` … `row11_mode0_threshold_int_max` |
| 12–16 | `tests/configs.rs` | `row12_mode1_zero_iterations` … `row16_mode1_threshold_int_max` |
| 17–21 | `tests/configs.rs` | `row17_mode2_zero_iterations` … `row21_mode2_threshold_int_max` |
| 22 | `tests/configs.rs` | `row22_mode_default_from_above` |
| 23 | `tests/configs.rs` | `row23_mode_default_from_below` |
| 24 | `tests/configs.rs` | `row24_seed_boundaries_all_modes` |
| 25 | `tests/configs.rs` | `row25_threshold_exactly_at_produced_value` |
| 26 | `tests/configs.rs` | `row26_max_iterations` |
| 27 | `tests/configs.rs` | `row27_full_random_fuzz` |
| 28 | `tests/configs.rs` | `row28_repeated_invocations_are_independent` |

## Beyond the table: exhaustive sweeps

`tests/exhaustive.rs` goes past random sampling and compares return values over
contiguous ranges (stdout is pinned separately — see the file's module comment):

| test | space covered |
|------|---------------|
| `exhaustive_all_valid_seeds` | **all 65 536 valid `seed` values** × 5 mode paths × 6 threshold regimes (≈ 2.0 M call pairs) |
| `exhaustive_contiguous_iteration_counts` | **every `iterations` value in `[0, 4096]`** × 4 mode paths × 5 (seed, threshold) pairs |
| `exhaustive_range_check_boundaries` | ±3 around every range-check edge, in `iterations` × `seed` simultaneously × 5 modes × 4 thresholds |
| `exhaustive_contiguous_thresholds` | **every `threshold` in `[-5, 3010]`** (the full span of reachable produced values) × 4 modes × 8 seeds |
| `exhaustive_operation_callbacks` | every `value` in `[-70000, 70000]` plus every power-of-two edge and both overflow extremes, for all three callbacks |

## Suite sensitivity (mutation testing)

Passing tests only mean something if they can fail. `mutation_check.sh` injects
19 realistic translation bugs into `src/lib.rs` one at a time, rebuilds, and runs
the suite. **All 19 were caught; 0 survived.** Mutations covered wrong
arithmetic constants, `>` vs `>=` on both range checks, swapped check order,
swapped/wrong mode dispatch, `<` vs `<=` on the store predicate, a changed
modulus, an off-by-one max-count cutoff, wrong error codes, an off-by-one
summation bound, three dropped log lines and one altered log message.

## Final result

Run `./run_all.sh` to reproduce. Result: **52 tests pass in all 4
configurations** (`{release, dev}` × `{--no-default-features, default}`), and the
`nm -D` symbol diff is empty.
