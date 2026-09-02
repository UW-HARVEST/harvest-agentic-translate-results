# CONFIGS.md — Configuration-surface table (Phase A → gates Phase B)

Mechanically derived from the branches the C source actually takes.

## Axes the C code branches on

Public entry points (`nm -D`, and `include/lib.h` declares only the last one):

- `process_value(int value, int unused_param, void *unused_context)` — lowest level
- `double_value(int value, int unused_param, void *unused_context)` — lowest level
- `triple_value(int value, int unused_param, void *unused_context)` — lowest level
- `gotomach(int iterations, int seed, int mode, int threshold)` — composed pipeline

There are no compile-time options: `c_src/CMakeLists.txt` defines no macros and
`lib.c` contains no `#ifdef` other than header guards inside libc. The Rust crate
declares **no `[features]`**, so the only feature configuration is the default
(empty) one — see the "feature combinations" section at the bottom.

Runtime "option" axes:

| axis | source of the branch | distinct values |
|---|---|---|
| `mode` | `switch (mode)` in `gotomach` selects `state->operation` | `0` → `process_value`, `1` → `double_value`, `2` → `triple_value`, `default` → `process_value` + `[WARNING]` |
| `iterations` | `if (iterations < 0 \|\| iterations > UINT16_MAX)`; `malloc(iterations*4)` twice; loop trip count; `state->capacity` | `0` (empty), `1` (one), `2..65534` (many), `65535` (max) |
| `seed` | `if (seed < 0 \|\| seed > UINT16_MAX)`; initial `current_value` | `0`, `1`, mid, `65535` |
| `threshold` | `if (temp_buffer[i] < threshold)` decides whether a value is appended to `state->results`, i.e. how much of `count` fills up | `INT_MIN` (store none), low (store none), interleaving (store some), high (store all), `INT_MAX` (store all) |
| `state->count` ceiling | `if (state->count >= UINT16_MAX) break;` | not reached vs reached |
| fixed-point / cycle shape of `current_value` | `current_value = temp_buffer[i] % 1000` feeding the next `state->operation` call | per-`mode` orbit: `+10` walks and wraps mod 1000; `*2` doubles mod 1000; `*3` triples mod 1000 — each has different cycles and different negative/positive behaviour |
| `unused_param` / `unused_context` of the op functions | `(void)`-cast, never read | `0` / arbitrary int; `NULL` / arbitrary non-null pointer |
| `value` of the op functions | plain arithmetic, C signed overflow wraps in practice | `0`, `±1`, small, `INT_MAX`, `INT_MIN`, random full-range |

## Table — one row per meaningful combination

Each row is exercised with **many randomized inputs** (fixed-seed xorshift PRNG,
so runs are reproducible) over the free variables of that row, comparing the C
`.so` and the Rust `.so` return values byte-for-byte through `libloading`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `process_value` | `value` = full `i32` range, randomized; `unused_param` randomized; `unused_context` = `NULL` | [x] |
| C2 | `process_value` | `value` at boundaries `{0, 1, -1, 9, 10, INT_MAX-10, INT_MAX-9, INT_MAX, INT_MIN, INT_MIN+1}`; `unused_context` = non-null garbage | [x] |
| C3 | `double_value` | `value` = full `i32` range, randomized; `unused_context` = `NULL` | [x] |
| C4 | `double_value` | `value` at overflow boundaries `{INT_MAX, INT_MAX/2, INT_MAX/2+1, INT_MIN, INT_MIN/2, INT_MIN/2-1}`; `unused_context` = non-null garbage | [x] |
| C5 | `triple_value` | `value` = full `i32` range, randomized; `unused_context` = `NULL` | [x] |
| C6 | `triple_value` | `value` at overflow boundaries `{INT_MAX, INT_MAX/3, INT_MAX/3+1, INT_MIN, INT_MIN/3, INT_MIN/3-1}`; `unused_context` = non-null garbage | [x] |
| C7 | all three ops | same `value` fed to all three; `unused_param` swept over `{INT_MIN, -1, 0, 1, INT_MAX}` to prove it is ignored identically | [x] |
| C8 | `gotomach` | `mode = 0`, `iterations = 0` (empty shape), random `seed`, random `threshold` | [x] |
| C9 | `gotomach` | `mode = 1`, `iterations = 0`, random `seed`, random `threshold` | [x] |
| C10 | `gotomach` | `mode = 2`, `iterations = 0`, random `seed`, random `threshold` | [x] |
| C11 | `gotomach` | `mode` = out-of-range (default branch), `iterations = 0`, random `seed`/`threshold` | [x] |
| C12 | `gotomach` | `mode = 0`, `iterations = 1` (one shape), random `seed`, random `threshold` | [x] |
| C13 | `gotomach` | `mode = 1`, `iterations = 1`, random `seed`, random `threshold` | [x] |
| C14 | `gotomach` | `mode = 2`, `iterations = 1`, random `seed`, random `threshold` | [x] |
| C15 | `gotomach` | `mode` out-of-range, `iterations = 1`, random `seed`, random `threshold` | [x] |
| C16 | `gotomach` | `mode = 0`, `iterations` random in `2..=512` (many), random `seed`, `threshold = INT_MIN` (append **none**, `count == 0`) | [x] |
| C17 | `gotomach` | `mode = 0`, `iterations` random in `2..=512`, random `seed`, `threshold = INT_MAX` (append **all**) | [x] |
| C18 | `gotomach` | `mode = 0`, `iterations` random in `2..=512`, random `seed`, `threshold` random in the *interleaving* band `-2000..=4000` (append **some**) | [x] |
| C19 | `gotomach` | `mode = 1`, `iterations` random in `2..=512`, `threshold = INT_MIN` | [x] |
| C20 | `gotomach` | `mode = 1`, `iterations` random in `2..=512`, `threshold = INT_MAX` | [x] |
| C21 | `gotomach` | `mode = 1`, `iterations` random in `2..=512`, interleaving `threshold` | [x] |
| C22 | `gotomach` | `mode = 2`, `iterations` random in `2..=512`, `threshold = INT_MIN` | [x] |
| C23 | `gotomach` | `mode = 2`, `iterations` random in `2..=512`, `threshold = INT_MAX` | [x] |
| C24 | `gotomach` | `mode = 2`, `iterations` random in `2..=512`, interleaving `threshold` | [x] |
| C25 | `gotomach` | `mode` out-of-range (randomized invalid ints incl. `INT_MIN`/`INT_MAX`), `iterations` random in `2..=512`, `threshold` random full-range | [x] |
| C26 | `gotomach` | fully randomized valid tuple: `mode ∈ {0,1,2}`, `iterations ∈ 0..=65535`, `seed ∈ 0..=65535`, `threshold` = full `i32` range | [x] |
| C27 | `gotomach` | `iterations = 65535` (**max capacity**), each `mode`, `threshold = INT_MAX` → drives the `state->count >= UINT16_MAX` `[WARNING] Reached maximum count` early `break` | [x] |
| C28 | `gotomach` | `iterations = 65535`, each `mode`, `threshold = INT_MIN` → max trip count with `count == 0` (ceiling never reached) | [x] |
| C29 | `gotomach` | `iterations = 65535`, each `mode`, interleaving `threshold` → partial fill at max capacity | [x] |
| C30 | `gotomach` | `iterations = 65534` (one below the ceiling-triggering count), `threshold = INT_MAX`, each `mode` → `count == 65534`, ceiling **not** reached | [x] |
| C31 | `gotomach` | `seed` boundary sweep `{0, 1, 999, 1000, 1001, 65534, 65535}` × each `mode`, `iterations = 64`, `threshold = INT_MAX` — exercises the `% 1000` orbit entry points | [x] |
| C32 | `gotomach` | `threshold` boundary sweep around the exact values the ops emit: `{9,10,11}` (`+10`), `{0,1,2}`, `{1998,1999,2000,2001}` (`*2`), `{2997,2998,2999,3000}` (`*3`) × each `mode`, `seed ∈ {0,1,999,1000}` | [x] |
| C33 | `gotomach` | repeated back-to-back calls in one process (state must not leak between calls): 200 randomized valid tuples called alternately on C then Rust, and interleaved C/Rust/C to detect cross-call state | [x] |
| C34 | `gotomach` | stdout byte-for-byte: log line sequence captured via `dup2` for each of the reachable log paths (valid run, invalid iterations, invalid seed, invalid mode, max count) | [x] |
| C35 | `gotomach` + ops | mixed usage: the exported op functions called directly with the same values `gotomach` feeds them internally (`seed`, then `x % 1000` orbit values), verifying the low-level entry points agree with the composed pipeline | [x] |

## Feature combinations (Phase D)

`translation/Cargo.toml` declares no `[features]` table, so the complete set of
feature combinations is:

| combo | command |
|---|---|
| default (no features) | `cargo test --release` |
| `--no-default-features` | `cargo test --release --no-default-features` |
| `--all-features` | `cargo test --release --all-features` |

All three are identical configurations here, and all three are run by
`tests/run_all_features.sh`. The `.so` under test is always the **release**
`cdylib` (`crate-type = ["cdylib"]`, `panic = "abort"`), matching how an external
consumer links it; a debug `.so` is additionally built and diffed for symbols.

## Row → test mapping (Phase B)

Rows C1–C33 and C35 are in `tests/phase_b_valid.rs`; row C34 lives in its own
binary, `tests/phase_b_stdout.rs`, because its fd-1 redirection is process-wide
and must not run concurrently with tests that make the libraries log.

| row | test |
|---|---|
| C1 | `c1_process_value_random_full_range` |
| C2 | `c2_process_value_boundaries` |
| C3 | `c3_double_value_random_full_range` |
| C4 | `c4_double_value_boundaries` |
| C5 | `c5_triple_value_random_full_range` |
| C6 | `c6_triple_value_boundaries` |
| C7 | `c7_all_ops_same_value_unused_param_swept` |
| C8 | `c8_empty_mode0` |
| C9 | `c9_empty_mode1` |
| C10 | `c10_empty_mode2` |
| C11 | `c11_empty_mode_invalid` |
| C12 | `c12_one_mode0` |
| C13 | `c13_one_mode1` |
| C14 | `c14_one_mode2` |
| C15 | `c15_one_mode_invalid` |
| C16 | `c16_many_mode0_threshold_min` |
| C17 | `c17_many_mode0_threshold_max` |
| C18 | `c18_many_mode0_threshold_interleaving` |
| C19 | `c19_many_mode1_threshold_min` |
| C20 | `c20_many_mode1_threshold_max` |
| C21 | `c21_many_mode1_threshold_interleaving` |
| C22 | `c22_many_mode2_threshold_min` |
| C23 | `c23_many_mode2_threshold_max` |
| C24 | `c24_many_mode2_threshold_interleaving` |
| C25 | `c25_many_mode_invalid_threshold_any` |
| C26 | `c26_fully_randomized_valid_domain` |
| C27 | `c27_max_capacity_threshold_max_triggers_ceiling` |
| C28 | `c28_max_capacity_threshold_min` |
| C29 | `c29_max_capacity_threshold_interleaving` |
| C30 | `c30_one_below_ceiling` |
| C31 | `c31_seed_boundary_sweep` |
| C32 | `c32_threshold_boundary_sweep` |
| C33 | `c33_repeated_and_interleaved_calls` |
| C34 | `c34_stdout_byte_identical` (`tests/phase_b_stdout.rs`) |
| C35 | `c35_low_level_ops_match_composed_pipeline` |

**Status: 35/35 `CONFIGS.md` rows pass across their randomized inputs, under
every feature combination and both build profiles.**

## How to run

```bash
# one profile / one feature set
cd translation && cargo test --release

# the whole matrix (rebuilds the C .so, diffs symbols, runs every combo)
cd translation && ./tests/run_all_features.sh
```
