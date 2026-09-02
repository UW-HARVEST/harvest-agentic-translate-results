# CONFIGS.md — configuration-surface table

## Axes the C code actually branches on

Derived from `c_src/include/lib.h` (the whole public API is
`int jumpnode(int a, int b, int c, int d);`) and every `if` / `switch` / loop
condition in `c_src/src/lib.c`.

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| `operation_mode` (arg 1) | `0001`, `0002`, `0003`, `0004`, everything else (`default:`) | `switch (operation_mode)` |
| `node_id` (arg 2) | only via `find_node_by_id`, which always fails (`node_count == 0`); in mode `0003` it feeds `%d`, so its **decimal width and sign** matter | `find_node_by_id`, `sprintf` |
| `depth` (arg 3) | mode `0001`: loop bound; mode `0002`: `start_offset` for `process_backward`; mode `0003`: `%d` width/sign; mode `0004`: `1.0 + depth*0.1` | all four cases |
| `flags` (arg 4) | mode `0002`: `+ 16*flags`; mode `0003`: `+ (flags & 0177)`; modes `0001`/`0004`: unused | cases `0002`, `0003` |
| `node_count` global | `0` (only reachable state), `> 2` gate in mode `0004` | `initialize_test_data` is never called |
| formatted-string shape (mode `0003`) | total `strlen` of `"Node_<id>_Depth_<depth>"`: 13 (both single-digit) … 34 (both `INT_MIN`) | `compute_size_metric` |

No compile-time `#ifdef`s, no runtime option setters, no global config struct, no
byte-order or element-type axes: the library has no state-mutating entry point,
so the only "configuration" is the four-argument cross-product.

## Cargo feature axis

`translation/Cargo.toml` declares **no `[features]` table** and no
`default` feature, so the only feature combination that exists is the empty one.
`cargo test --no-default-features` and plain `cargo test` compile identical code.
Both are run by `run_all.sh` for completeness.

## Rows

Cross-product of the axes, pruned to combinations the C treats differently.
Every row is driven with many seeded-random inputs, not one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `jumpnode` | mode `0001`, random `node_id`, `depth = 0`, random `flags` → null-node path | [x] |
| 2 | `jumpnode` | mode `0001`, random `node_id`, `depth > 0` (1..64), random `flags` | [x] |
| 3 | `jumpnode` | mode `0001`, random `node_id`, `depth < 0`, random `flags` | [x] |
| 4 | `jumpnode` | mode `0001`, `node_id ∈ {1..7}` (the ids `initialize_test_data` *would* have made), `depth ∈ {0,1,2,3,10}` | [x] |
| 5 | `jumpnode` | mode `0001`, extremes: `node_id`/`depth`/`flags` ∈ {`INT_MIN`,`-1`,`0`,`1`,`INT_MAX`} full cross-product | [x] |
| 6 | `jumpnode` | mode `0002`, random `node_id`, `depth ∈ 0..15` (in-range `start_offset` for `process_backward`) | [x] |
| 7 | `jumpnode` | mode `0002`, random `node_id`, `depth = 16` (`start == ptr`, empty backward walk) | [x] |
| 8 | `jumpnode` | mode `0002`, random `node_id`, `depth > 16` (`start > ptr`, loop skipped) | [x] |
| 9 | `jumpnode` | mode `0002`, random `node_id`, `depth < 0` (`start` below buffer) | [x] |
| 10 | `jumpnode` | mode `0002`, `flags` chosen so `16 * flags` overflows `int` (`flags` near `INT_MAX/16`) | [x] |
| 11 | `jumpnode` | mode `0002`, extremes cross-product of `node_id`/`depth`/`flags` | [x] |
| 12 | `jumpnode` | mode `0003`, both `node_id` and `depth` single-digit non-negative (shortest string, len 13) | [x] |
| 13 | `jumpnode` | mode `0003`, `node_id` and `depth` sweeping every decimal width 1..10, positive | [x] |
| 14 | `jumpnode` | mode `0003`, `node_id` and `depth` negative (extra `-` byte in `%d`) | [x] |
| 15 | `jumpnode` | mode `0003`, `node_id = INT_MIN`, `depth = INT_MIN` (longest string, len 34) | [x] |
| 16 | `jumpnode` | mode `0003`, `node_id = 0` and/or `depth = 0` (the `magnitude == 0` digit path) | [x] |
| 17 | `jumpnode` | mode `0003`, powers-of-ten and ±1 neighbours for `node_id`/`depth` (digit-count boundaries) | [x] |
| 18 | `jumpnode` | mode `0003`, `flags` random full-range → `flags & 0177` | [x] |
| 19 | `jumpnode` | mode `0003`, `flags` ∈ {`0`, `0177`, `0200`, `0377`, `-1`, `INT_MIN`, `INT_MAX`} (mask boundaries, incl. negative) | [x] |
| 20 | `jumpnode` | mode `0003`, full random cross-product of `node_id`/`depth`/`flags` | [x] |
| 21 | `jumpnode` | mode `0004`, random `node_id`, `depth = 0` | [x] |
| 22 | `jumpnode` | mode `0004`, random `node_id`, `depth > 0` (`1.0 + depth*0.1` scale-up) | [x] |
| 23 | `jumpnode` | mode `0004`, `depth = -10` (`1.0 + depth*0.1 == 0.0` exactly) | [x] |
| 24 | `jumpnode` | mode `0004`, `depth < -10` (negative scale factor) | [x] |
| 25 | `jumpnode` | mode `0004`, `depth` huge (`INT_MAX`, `INT_MIN`) → `safe_double_to_int` clamp range | [x] |
| 26 | `jumpnode` | mode `0004`, extremes cross-product of `node_id`/`depth`/`flags` | [x] |
| 27 | `jumpnode` | `default:` arm, `operation_mode` random outside `1..4`, random other args | [x] |
| 28 | `jumpnode` | `default:` arm, `operation_mode` ∈ {`0`,`5`,`-1`,`INT_MIN`,`INT_MAX`,`0177`,`0200`,`0377`} | [x] |
| 29 | `jumpnode` | repeated / interleaved calls in one process across all modes — checks no hidden global state (`node_count`) drifts between calls or between the two `.so`s | [x] |
| 30 | `jumpnode` | fully unconstrained fuzz: all four args uniform over the whole `int` range, 200 000 seeded iterations | [x] |

All 30 rows are implemented in `tests/differential.rs` as `phase_b_*` tests and
pass against both `.so`s.

## Feature / profile matrix actually run

`./run_all.sh` extracts the feature list from `Cargo.toml` (currently empty),
builds the power set, and crosses it with both cdylib build profiles:

| # | cdylib profile | cargo invocation | result |
|---|----------------|------------------|--------|
| 1 | release (overflow checks off) | `cargo test --release` | 39/39 pass |
| 2 | release | `cargo test --release --no-default-features` | 39/39 pass |
| 3 | debug (overflow checks ON) | `cargo test --release` | 39/39 pass |
| 4 | debug | `cargo test --release --no-default-features` | 39/39 pass |

The debug-cdylib rows matter independently of features: they enable Rust's
arithmetic overflow checks. C wraps silently on signed overflow, so any place the
translation used plain `+`/`*` where the C can overflow would panic there instead
of wrapping. Both debug rows pass, so no reachable path has that divergence.

`FFI_TEST_PROFILE`, `FFI_TEST_NO_DEFAULT_FEATURES` and `FFI_TEST_FEATURES` are
read by `tests/common/mod.rs` and forwarded to the nested cdylib build, so the
`.so` under test is always built with the same configuration as the test binary.
