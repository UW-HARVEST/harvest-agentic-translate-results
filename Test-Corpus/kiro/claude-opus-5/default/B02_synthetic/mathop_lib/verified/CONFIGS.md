# CONFIGS.md — configuration / valid-input surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Axes the C code actually branches on

There are **no** compile-time options: no `#ifdef` other than the include
guards implied by the standard headers, no build flags in
`c_src/CMakeLists.txt` beyond `SHARED`, and no `[features]` in
`translation/Cargo.toml`. Every axis below is therefore a **runtime input**
axis.

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| A1 `Operation` selector | `1 OP_ADD`, `2 OP_MULTIPLY`, `3 OP_SUBTRACT`, `4 OP_DIVIDE`, `5 OP_MODULO`, and *anything else* → `default:` | `select_operation` `switch` (lines 89-102) |
| A2 divisor zero-ness | `b == 0` vs `b != 0` | `divide_operation` / `modulo_operation` line 75 / 82 |
| A3 operand sign / magnitude | positive, negative, `0`, `INT_MIN`, `INT_MAX`, `-1` (C `/` and `%` truncate toward zero, so sign matters) | `a / b`, `a % b`, `% 5`, `% 128`, `% 100` |
| A4 history-pointer state | `*history == NULL` (bootstrap) vs already allocated | line 122 |
| A5 history fill level | `*history_count < 10` vs `>= 10` | line 127 |
| A6 `allocate_results` count | `0`, `1`, `< 10`, `10`, `> 10`, negative, `INT_MAX` | line 113 |
| A7 `is_valid_operation` char class | `0`, `< '1'`, `'1'`..`'5'`, `> '5'`, negative (`char` is signed) | line 53 |
| A8 `mathop` `param1` | selects the (dead) validation char via `% 128`, and is the first operand | lines 141-144, 150 |
| A9 `mathop` `param3` | `param3 % 5 + 1` picks `selected_op`; negative `param3` yields an out-of-range op | line 147 |
| A10 `mathop` `param4` | `(param4 + 1) % 5 + 1` picks `second_op`, **and** `param4` is the second operand of the second computation | lines 154-157 |
| A11 call sequence position | mathop's `static` history accumulates 2 entries per call, saturating at 10 (i.e. call #1..#5 fill, #6+ saturate) | lines 138-139 |
| A12 `time_t` shift result | `time() >> 29`, then `% 100` | `get_computation_timestamp`, line 168 |

## Entry points (all 12, lowest level first)

`add_operation`, `multiply_operation`, `subtract_operation`,
`divide_operation`, `modulo_operation` (leaf arithmetic) →
`is_valid_operation`, `get_operation_priority`, `get_computation_timestamp`,
`allocate_results`, `select_operation` (leaf helpers) →
`perform_computation_with_history` (composes selector + history) →
`mathop` (the only header-declared entry point; composes everything twice
over shared `static` state).

`select_operation` is exercised by **calling the returned function pointer**
and by **identifying** it against each library's own exported
`{add,multiply,subtract,divide,modulo}_operation` address — a raw pointer
comparison across the two `.so`s would be meaningless.

## Table — one row per combination the C treats differently

Every row is driven with **many randomized inputs** (`SplitMix64`, fixed
seed `0x5EED_1234_ABCD_F00D`) plus the boundary values named in the row.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `add_operation` | randomized full-range `a`,`b`; boundaries `0/1/-1/INT_MIN/INT_MAX`; overflow pairs; `unused_param` varied (must be ignored) | [x] |
| 2 | `multiply_operation` | randomized full-range; boundaries; overflowing products; `unused_param` varied | [x] |
| 3 | `subtract_operation` | randomized full-range; boundaries; `INT_MIN - 1` / `INT_MAX - (-1)` overflow; `unused_param` varied | [x] |
| 4 | `divide_operation` | `b != 0`, randomized full range incl. mixed signs (truncation toward zero); `a == INT_MIN` with `b != -1`; `b == 1`, `b == -1` with `a != INT_MIN` | [x] |
| 5 | `divide_operation` | `b == 0` sentinel path, `a` randomized (see ERRORS row 4) | [x] |
| 6 | `modulo_operation` | `b != 0`, randomized full range incl. mixed signs (result takes sign of `a`); `a == INT_MIN` with `b != -1`; `b == 1`, `b == -1` with `a != INT_MIN` | [x] |
| 7 | `modulo_operation` | `b == 0` sentinel path, `a` randomized (see ERRORS row 5) | [x] |
| 8 | `is_valid_operation` | **all 256** `char` bit patterns (`-128..=127`), i.e. every class of axis A7 exhaustively | [x] |
| 9 | `get_operation_priority` | in-range ops `1..=5` | [x] |
| 10 | `get_operation_priority` | out-of-range / negative / overflowing ops: `0`, `6`, `-1`, `-3`, `INT_MIN`, `INT_MAX`, randomized full range (`op * 10` wraps) | [x] |
| 11 | `get_computation_timestamp` | no inputs; both libs called back-to-back, values must agree (`time() >> 29`) | [x] |
| 12 | `allocate_results` | `count` = `1`, `10`, `11`, `100`: non-NULL, and all `count*24` bytes zeroed | [x] |
| 13 | `select_operation` | each in-range op `1..=5`: returned pointer identified against the exporting library's own symbol, **and** invoked on randomized `(a,b)` with results compared | [x] |
| 14 | `select_operation` | out-of-range op (`0`, `6`, `-1`, `INT_MIN`, `INT_MAX`, randomized): must identify as `add_operation` and behave as addition | [x] |
| 15 | `perform_computation_with_history` | A4 = bootstrap (`*history == NULL`), each op `1..=5`, randomized `(a,b)`: return value **and** all 10 freshly-callocated slots compared byte-for-byte | [x] |
| 16 | `perform_computation_with_history` | A4 = caller-allocated buffer, A5 = `count` `0`, then repeatedly appended `1..9`; op cycled `1..=5`; whole 10-slot buffer compared byte-for-byte after each call | [x] |
| 17 | `perform_computation_with_history` | A4 = caller-allocated, A5 = `count == 10` exactly (boundary: guard fails, nothing written, count unchanged) | [x] |
| 18 | `perform_computation_with_history` | A4 = caller-allocated, A5 = `count > 10` (`11`, `100`): still no write, count unchanged | [x] |
| 19 | `perform_computation_with_history` | A1 out-of-range op with A4 bootstrap and with caller buffer (silently adds) | [x] |
| 20 | `perform_computation_with_history` | op = `OP_DIVIDE`/`OP_MODULO` with `b == 0`: `0` recorded in the slot, `status = 0` | [x] |
| 21 | `perform_computation_with_history` | full 10-append sequence from a bootstrap history, op driven by randomized data, buffer + count compared after **every** step (saturation reached in-sequence) | [x] |
| 22 | `mathop` | A9: `param3 % 5 + 1` == each of `1..=5` (positive `param3` = `0,1,2,3,4` + multiples), other params randomized | [x] |
| 23 | `mathop` | A9 negative: `param3` = `-1,-2,-3,-4,-5,-6,INT_MIN` → out-of-range `selected_op` in `-3..=0` → negative priority + addition | [x] |
| 24 | `mathop` | A10: `(param4+1) % 5 + 1` == each of `1..=5`; includes `param4 == -1` (→ `second_op == 1`) and `param4 == INT_MAX` (overflow) | [x] |
| 25 | `mathop` | A8: `param1` chosen so `(char)(param1 % 128)` is **valid** (`'1'`..`'5'`, i.e. 49..53 and 49..53 + 128k) | [x] |
| 26 | `mathop` | A8: `param1` chosen so `(char)(param1 % 128)` is **invalid** (0, `'0'`, `'6'`, 127, negative) | [x] |
| 27 | `mathop` | A2 inside `mathop`: `selected_op == OP_DIVIDE`/`OP_MODULO` with `param2 == 0`; `second_op == OP_DIVIDE`/`OP_MODULO` with `param4 == 0` | [x] |
| 28 | `mathop` | A3 boundaries: `param1`/`param2`/`param4` at `INT_MIN`, `INT_MAX`, `0`, `-1`, `1` in a cross-product (excluding the `SIGFPE` pair, covered by ERRORS row 24) | [x] |
| 29 | `mathop` | A11: 12 consecutive calls in one process on each library, so the shared static history bootstraps, fills, and saturates; every return value compared | [x] |
| 30 | `mathop` | 2000 fully randomized `(param1..param4)` quadruples, `SIGFPE` pair filtered | [x] |
| 31 | full pipeline | `select_operation` → returned pointer → `perform_computation_with_history` on the **same** library, chained over randomized data, i.e. the composed path rather than per-wrapper calls | [x] |
| 32 | `ComputationResult` ABI | 24-byte size / 8-byte align / field offsets `0,8,16`, padding bytes stay zero after a write (verified through the FFI buffer, byte-for-byte) | [x] |

## How the rows are driven

`tests/phase_b_leaf.rs` covers rows 1-14, `tests/phase_b_composed.rs` covers
rows 15-32. Every row uses the fixed-seed `SplitMix64` in `tests/common/mod.rs`
(`SEED = 0x5EED_1234_ABCD_F00D`, per-row seed = `SEED ^ row`), drawing from a
mixed distribution: small values, values near `INT_MIN`/`INT_MAX`, values in
`0..128` (the `% 128` band), a `±500` band, and the unbiased full `i32` range.

Both libraries are reached only through `dlopen`/`dlsym` on their `.so` files —
no Rust function is called directly, so the `#[unsafe(no_mangle)] extern "C"`
wrappers are part of what is under test.

## Build-configuration caveat

`cargo test` does **not** rebuild the `cdylib`, because the integration tests
`dlopen` it instead of linking it. Running `cargo test` after editing
`src/lib.rs` would otherwise verify a stale `.so`; `tests/common/mod.rs`
compares mtimes and fails loudly instead. `scripts/verify_all.sh` always builds
before testing.
