# CONFIGS.md — Configuration-surface table (Phase A, gates Phase B)

## Axes the C actually branches on

Derived from the `if` / `switch` / comparison sites in `c_src/src/lib.c`:

| axis | values the code distinguishes | source site |
|------|-------------------------------|-------------|
| entry point | `gotomach`, `process_value`, `double_value`, `triple_value` | the 4 exported `T` symbols |
| `mode` (operation selector) | `0` → `process_value` (`v+10`), `1` → `double_value` (`v*2`), `2` → `triple_value` (`v*3`), anything else → default/`process_value` + `[WARNING]` | `switch (mode)` L126–140 |
| `iterations` (workload shape) | `0` (empty, loop never runs), `1` (single), small (`2..64`), large (`~1000`), max (`65535` — also the only way to reach `count >= UINT16_MAX`) | `for (i=0; i<iterations; i++)` L163, `malloc` L83/L149 |
| `seed` (start value) | `0` (boundary low), `1`, mid (`1..65534`), `65535` (boundary high) | `current_value = seed` L162 |
| `threshold` (append filter) | `INT_MIN` (nothing appended → `count==0`, sum `0`), very low (nothing appended), interleaved/mid (SOME appended — the value-dependent path), very high / `INT_MAX` (everything appended) | `if (temp_buffer[i] < threshold)` L172 |
| `results` fill level | `count == 0`, `0 < count < capacity`, `count == capacity` (only when every element passes the filter) | `state->results[state->count++]` L173, `is_valid_state` L50 |
| feedback recurrence | `current_value = temp_buffer[i] % 1000` — the operation output is fed back mod 1000, so values are bounded and become periodic; different `mode`s reach different cycles | L176 |
| `unused_context` (helpers) | `NULL` vs. non-`NULL` (must be ignored either way) | `(void)unused_context` L61/L67/L73 |
| `unused_param` (helpers) | `0`, positive, negative, `INT_MIN`/`INT_MAX` (must be ignored) | `(void)unused_param` L60/L66/L72 |
| helper input magnitude | small, `INT_MAX`/`INT_MIN` (signed-overflow behaviour of `+10`, `*2`, `*3`) | L62/L68/L74 |
| stdout side effects | every path prints at least `[INFO] Starting gotomach function`; the mode-default path adds a `[WARNING]`; success adds `[INFO] Processing completed successfully` | `LOG_MSG` L112–189 |

## Build/feature axis

`translation/Cargo.toml` has **no `[features]` table**. The default build is the
only configuration; `--no-default-features` is byte-identical to it. There is
**no binary target**, so there is no driver-stdout comparison — stdout is
compared by redirecting fd 1 around each `.so` call instead.

## Configuration rows (each checked off only after randomized differential runs)

Lowest-level entry points (rows 1–8) come first, then the composed `gotomach`
pipeline (rows 9–30).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `process_value` | randomized `value` over full `i32` range, `unused_param = 0`, `unused_context = NULL` | [x] |
| 2 | `process_value` | boundary `value` ∈ {`INT_MIN`, `-1`, `0`, `1`, `INT_MAX-10`, `INT_MAX`}; randomized junk `unused_param`; non-`NULL` `unused_context` | [x] |
| 3 | `double_value` | randomized `value` over full `i32` range, `unused_param = 0`, `unused_context = NULL` | [x] |
| 4 | `double_value` | boundary `value` ∈ {`INT_MIN`, `INT_MIN/2`, `-1`, `0`, `1`, `INT_MAX/2`, `INT_MAX`}; randomized junk `unused_param`; non-`NULL` `unused_context` | [x] |
| 5 | `triple_value` | randomized `value` over full `i32` range, `unused_param = 0`, `unused_context = NULL` | [x] |
| 6 | `triple_value` | boundary `value` ∈ {`INT_MIN`, `INT_MIN/3`, `-1`, `0`, `1`, `INT_MAX/3`, `INT_MAX`}; randomized junk `unused_param`; non-`NULL` `unused_context` | [x] |
| 7 | all 3 helpers | called through an `operation_fn` function pointer obtained from the `.so` (same call shape `gotomach` uses internally), values in the `0..65535` + `%1000` domain that `gotomach` actually feeds them | [x] |
| 8 | all 3 helpers | `unused_context` = dangling/garbage non-null pointer, `unused_param` = `INT_MIN`/`INT_MAX` — must be ignored, result depends only on `value` | [x] |
| 9 | `gotomach` | `mode=0`, `iterations=0` (empty workload), randomized `seed`/`threshold` | [x] |
| 10 | `gotomach` | `mode=1`, `iterations=0`, randomized `seed`/`threshold` | [x] |
| 11 | `gotomach` | `mode=2`, `iterations=0`, randomized `seed`/`threshold` | [x] |
| 12 | `gotomach` | invalid `mode` (out-of-range enum), `iterations=0`, randomized `seed`/`threshold` | [x] |
| 13 | `gotomach` | `mode=0`, `iterations=1` (single element), randomized `seed`/`threshold` | [x] |
| 14 | `gotomach` | `mode=1`, `iterations=1`, randomized `seed`/`threshold` | [x] |
| 15 | `gotomach` | `mode=2`, `iterations=1`, randomized `seed`/`threshold` | [x] |
| 16 | `gotomach` | `mode=0`, small `iterations` (2..64), randomized `seed`, `threshold` = `INT_MIN` (nothing appended, `count==0`) | [x] |
| 17 | `gotomach` | `mode=1`, small `iterations`, `threshold` = `INT_MIN` | [x] |
| 18 | `gotomach` | `mode=2`, small `iterations`, `threshold` = `INT_MIN` | [x] |
| 19 | `gotomach` | `mode=0`, small `iterations`, `threshold` = `INT_MAX` (everything appended, `count==capacity`) | [x] |
| 20 | `gotomach` | `mode=1`, small `iterations`, `threshold` = `INT_MAX` | [x] |
| 21 | `gotomach` | `mode=2`, small `iterations`, `threshold` = `INT_MAX` | [x] |
| 22 | `gotomach` | `mode=0`, small `iterations`, `threshold` in the interleaving band (`0..3100`) so only SOME elements are appended — the value-dependent partial-fill path | [x] |
| 23 | `gotomach` | `mode=1`, small `iterations`, interleaving `threshold` | [x] |
| 24 | `gotomach` | `mode=2`, small `iterations`, interleaving `threshold` | [x] |
| 25 | `gotomach` | invalid `mode`, small `iterations`, interleaving `threshold` (default-op + partial fill + `[WARNING]` all at once) | [x] |
| 26 | `gotomach` | every `mode` ∈ {0,1,2,invalid} × `seed` boundary ∈ {`0`, `1`, `65534`, `65535`} × `threshold` ∈ {`INT_MIN`,`0`,`1000`,`INT_MAX`}, `iterations` = 8 (full pruned cross-product) | [x] |
| 27 | `gotomach` | large `iterations` (`500..2000`), all 4 modes, randomized `seed`/`threshold` — long recurrence, exercises the `%1000` cycle | [x] |
| 28 | `gotomach` | `iterations = 65535` (max accepted), `threshold = INT_MAX` → reaches `count >= UINT16_MAX` and prints `[WARNING] Reached maximum count`; all 4 modes | [x] |
| 29 | `gotomach` | `iterations = 65535`, `threshold` interleaving / `INT_MIN` (max workload without hitting the count cap); all 4 modes | [x] |
| 30 | `gotomach` | fully randomized fuzz over the whole accepted domain: `iterations ∈ 0..=1024`, `seed ∈ 0..=65535`, `mode ∈ INT_MIN..=INT_MAX`, `threshold ∈ INT_MIN..=INT_MAX`, fixed seed, many iterations; return value AND captured stdout both compared | [x] |
| 31 | `gotomach` | stdout byte-for-byte comparison on a representative set covering each distinct log-line combination (`[INFO]` only, `+[WARNING] Invalid mode`, `+[WARNING] Reached maximum count`, each `[ERROR]`) | [x] |

## Phase B result

All 31 rows PASS. Row → test mapping (`translation/tests/phase_b_valid.rs`):

| rows | test(s) |
|------|---------|
| 1-2   | `row01_process_value_randomized_full_i32`, `row02_process_value_boundaries` |
| 3-4   | `row03_double_value_randomized_full_i32`, `row04_double_value_boundaries` |
| 5-6   | `row05_triple_value_randomized_full_i32`, `row06_triple_value_boundaries` |
| 7     | `row07_helpers_via_operation_fn_pointer_in_gotomach_domain` |
| 8     | `row08_helpers_ignore_unused_param_and_context` |
| 9-12  | `row09_mode0_iterations0` … `row12_invalid_mode_iterations0` |
| 13-15 | `row13_mode0_iterations1` … `row15_mode2_iterations1` |
| 16-21 | `row16_…_int_min_nothing_appended` … `row21_…_int_max_everything_appended` |
| 22-25 | `row22_…_interleaving_threshold` … `row25_invalid_mode_small_interleaving_threshold` |
| 26    | `row26_full_cross_product_modes_seeds_thresholds` |
| 27    | `row27_large_iterations_all_modes` |
| 28-29 | `row28_max_iterations_threshold_int_max_reaches_count_cap`, `row29_max_iterations_other_thresholds` |
| 30    | `row30_fuzz_accepted_domain`, `row30b_fuzz_with_stdout_comparison` |
| 31    | `row31_stdout_byte_for_byte_every_log_combination` |

Every test dlopens BOTH `.so`s and compares the `int` return value; the rows
that assert log behaviour additionally compare the captured `printf` bytes.
All randomization is `SplitMix64` with per-row fixed seeds, so runs are
reproducible.
