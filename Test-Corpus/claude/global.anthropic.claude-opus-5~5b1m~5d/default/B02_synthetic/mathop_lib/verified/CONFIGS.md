# CONFIGS.md — Phase A: configuration-surface table

The mirror of `ERRORS.md`, for **valid** inputs. Derived mechanically from the
branches `c_src/src/lib.c` actually takes.

## The axes the C code branches on

This library has no init/config struct and no `#ifdef`; its "options" are the
argument values that steer control flow, plus the **caller-owned mutable state**
threaded through `ComputationResult** history` / `int* history_count`, plus the
**hidden `static` state** inside `mathop`.

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| **A. operation selector** | `1`=ADD, `2`=MUL, `3`=SUB, `4`=DIV, `5`=MOD, and `default` (anything else) | `select_operation` l.89-102 (6-way `switch`) |
| **B. divisor shape** | `b == 0` (guarded) vs `b != 0`; `b == 1`; `b == -1`; sign combinations of `a`,`b` (C `/` truncates toward zero, `%` takes the dividend's sign) | `divide_operation` l.74-79, `modulo_operation` l.81-86 |
| **C. arithmetic value shape** | `0`, `1`, `-1`, small +/-, `INT_MAX`, `INT_MIN`, and pairs that overflow `int` (add/sub/mul) | l.63, 67, 71 |
| **D. history pointer state** | `*history == NULL` (lazy allocate + count reset) vs `*history != NULL` (reuse caller's buffer) | l.122-125 |
| **E. history count state** | `< 10` (record) vs `>= 10` (silent drop); boundary `9` -> `10`; the count is also *reset to 0* on the NULL path | l.127-132 |
| **F. `mathop` static state** | fresh (first call: history NULL, count 0) / partially filled (count 2,4,6,8) / **saturated** (count 10, drops) — 2 records are appended per `mathop` call, so calls 1-5 fill and calls 6+ drop | l.138-139 + two calls at l.152/157 |
| **G. `mathop` validation char** | `(char)(param1 % 128)` lands in `'1'..'5'` (49..53) -> `is_valid` true, vs anything else -> false (dead fallback) | l.141-146 |
| **H. `mathop` first op** | `(param3 % 5) + 1` -> `1..5` for `param3 >= 0`; `0,-1,-2,-3` for `param3 < 0` (out-of-range -> ADD) | l.148 |
| **I. `mathop` second op** | `((param4 + 1) % 5) + 1` -> shifted by one relative to axis H; `param4 == INT_MAX` overflows | l.156 |
| **J. `char` domain** | all 256 signed-byte values `-128..=127`; in-range `'1'..'5'`, `0`, below, above | `is_valid_operation` l.52-55 |
| **K. allocation count** | `0`, `1`, `10` (the value the library itself uses), large-but-valid, `INT_MAX`, negative | `allocate_results` l.112-115 |
| **L. observable channel** | return value **and** the 4 `printf` lines on stdout (the "History entries" line exposes axis F) | l.168-171 |
| **M. time source** | `time(NULL) >> 29` — the same value must be produced by both `.so`s, and it is stored into every record's `timestamp` | l.105-110, l.129 |

`Cargo.toml` declares **no `[features]`**, so the feature axis is a single
point. Every row below is nevertheless verified in all four build configurations
(`dev`/`release` x default/`--no-default-features`) by `ci/verify_all.sh`.

## Configuration rows (cross-product, pruned to what the C distinguishes)

Every row is exercised with **many seeded-random inputs** (xorshift64\*, fixed
per-row seeds derived from `0x2545F4914F6CDD1D` — fully reproducible), not a
single hand-picked value, and compared byte-for-byte between the two `.so`s.
Inputs are biased towards corner values (`0`, `±1`, `INT_MIN`, `INT_MAX`, small
magnitudes) rather than drawn uniformly, so boundary paths are actually hit.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `is_valid_operation` | axis J: **exhaustive** all 256 `char` values `-128..=127` | `cfg_01_is_valid_operation_exhaustive` | [x] |
| 2 | `get_operation_priority` | axis A x C: every valid enum `1..5`, plus `0`, `6`, negatives, `INT_MIN`/`INT_MAX`, + random `i32` (overflow of `op*10`) | `cfg_02_get_operation_priority_random` | [x] |
| 3 | `add_operation` | axis C: random `i32` pairs incl. `0/1/-1/INT_MIN/INT_MAX` corners and wrapping overflow; 3rd arg varied (must be ignored) | `cfg_03_add_operation_random` | [x] |
| 4 | `multiply_operation` | axis C: random pairs incl. corners; products that overflow `int` | `cfg_04_multiply_operation_random` | [x] |
| 5 | `subtract_operation` | axis C: random pairs incl. corners; `INT_MIN - 1` style wrapping | `cfg_05_subtract_operation_random` | [x] |
| 6 | `divide_operation` | axis B x C: `b != 0`, all four sign combinations, `b == 1`, `b == -1`, `a == 0`, `INT_MAX/INT_MIN` operands (excluding the `INT_MIN / -1` C trap, ERRORS row 25) | `cfg_06_divide_operation_random` | [x] |
| 7 | `modulo_operation` | axis B x C: same shapes; verifies C's dividend-signed remainder | `cfg_07_modulo_operation_random` | [x] |
| 8 | `select_operation` | axis A: each of the 6 switch arms; the returned **function pointer is invoked** with random operands and its results compared (identity of behaviour, not address) | `cfg_08_select_operation_all_arms_invoked` | [x] |
| 9 | `get_computation_timestamp` | axis M: repeated calls; both `.so`s must return the identical `time_t >> 29` | `cfg_09_get_computation_timestamp` | [x] |
| 10 | `allocate_results` | axis K valid side: `count` = `0`, `1`, `2`, `10`, `64`, `1024`; result must be non-NULL and **fully zeroed** (`count*24` bytes) in both | `cfg_10_allocate_results_valid_counts_zeroed` | [x] |
| 11 | `perform_computation_with_history` | axis D=NULL (lazy alloc) x A: single call from a fresh `history = NULL`, `count` garbage; asserts return value, `count`, and the whole 24-byte record (`value`/`timestamp`/`status`) | `cfg_11_pcwh_lazy_alloc_all_ops` | [x] |
| 12 | `perform_computation_with_history` | axis D=non-NULL (caller buffer from `allocate_results`) x A x E `count<10`: appends into the caller's own buffer | `cfg_12_pcwh_caller_buffer_all_ops` | [x] |
| 13 | `perform_computation_with_history` | axis E boundary: drive count `0 -> 9 -> 10 -> 11...` in one lockstep sequence of 25 calls with random ops/operands; compares the **entire 10-slot array** (240 bytes) plus every return value at every step | `cfg_13_pcwh_fill_to_capacity_sequence` | [x] |
| 14 | `perform_computation_with_history` | axis D x E interaction: `*history == NULL` **while** `*history_count` is pre-set non-zero (5, 9, 10, 99) — the NULL branch must reset the count to 0 and then record at index 0 | `cfg_14_pcwh_null_history_with_stale_count` | [x] |
| 15 | `perform_computation_with_history` | axis A out-of-range x D x E: `op` = `0`, `6`, `-1`, `INT_MIN`, `INT_MAX` on a live buffer (falls back to ADD, still records) | `cfg_15_pcwh_out_of_range_op_records` | [x] |
| 16 | `perform_computation_with_history` | two independent caller histories interleaved (state is caller-owned, not global) — proves no cross-talk | `cfg_16_pcwh_two_independent_histories` | [x] |
| 17 | `mathop` | axis F fresh: the **very first** call on each `.so` (history NULL, count 0 -> 2); return value + all 4 stdout lines compared | `cfg_17_mathop_first_call_fresh_state` | [x] |
| 18 | `mathop` | axis F x H x I: lockstep sequence of 400 seeded-random 4-tuples driving the static state through fill (2 records per call) into **saturation** (count pinned at 10); asserts the counter is monotonic, even, never exceeds 10, and ends saturated; return value **and captured stdout** compared on every call | `cfg_18_mathop_long_random_sequence_stdout` | [x] |
| 19 | `mathop` | axis H: `param3 % 5` covering **all 5** positive residues -> `selected_op` `1..5` and priority `10..50`, with random `param1`/`param2`/`param4` | `cfg_19_mathop_all_first_ops` | [x] |
| 20 | `mathop` | axis I: `(param4 + 1) % 5` covering **all 5** residues -> `second_op` `1..5`. (Note: the second stage can never hit the `b == 0` guard, because its divisor *is* `param4`, and `param4 == 0` selects `second_op = 2` (MULTIPLY). The guard is reachable only in the first stage, via `param2 == 0`, which row 19 forces.) | `cfg_20_mathop_all_second_ops` | [x] |
| 21 | `mathop` | axis G true: `param1 % 128` in `49..=53` (`'1'..'5'`) so `is_valid` is true, x axis H | `cfg_21_mathop_valid_validation_char` | [x] |
| 22 | `mathop` | axis G false: `param1 % 128` outside `49..=53` (incl. `0`, negative residues) so the dead fallback fires, x axis H | `cfg_22_mathop_invalid_validation_char` | [x] |
| 23 | `mathop` | axis C corners: `param1..param4` drawn only from `{0, 1, -1, 2, -2, INT_MAX, INT_MIN, INT_MAX-1, INT_MIN+1}` — full 4-fold cross-product, **all 6561 combinations compared** (measured: 0 skipped, because no corner value of `param3` yields `param3 % 5 ∈ {3,4}`, so the trapping `idiv` of ERRORS row 25 is never reached from this set) | `cfg_23_mathop_corner_cross_product` | [x] |
| 24 | `mathop` | axis L: stdout bytes — all 4 `printf` lines incl. `%ld` timestamp formatting, asserted against a hand-computed expected line set, plus the `History entries` counter as it fills (2, 4, 6) and saturates (10) | `cfg_24_mathop_stdout_formatting`, `cfg_17_mathop_first_call_fresh_state`, `cfg_18_mathop_long_random_sequence_stdout` | [x] |
| 25 | full pipeline | composed low-level path: `select_operation` -> returned fn ptr -> `perform_computation_with_history` -> `allocate_results`, driven directly (not through `mathop`) with random ops/operands over a 30-call sequence | `cfg_25_composed_lowlevel_pipeline` | [x] |

## Where the rows are implemented

| file | rows | harness |
|------|------|---------|
| `tests/phase_b_configs.rs` | 1-16, 25 (17 tests) | default libtest |
| `tests/phase_stdout.rs` | 17-24 (+ ERRORS 20, 23, 24) | `harness = false`, strictly sequential |

Rows 17-24 need a captured `stdout`, and the libtest harness writes its own
progress lines (`test <name> ... ok`) to fd 1 — which corrupted a concurrently
captured region and produced a spurious extra line. Moving them into a
`harness = false` binary removed the interleaving entirely and, as a bonus, fixed
the call order so the hidden `static` counter's exact values (2, 4, 6, ... 10)
became assertable instead of merely comparable.

## Independent model cross-check

Beyond comparing the two `.so`s to each other, every `mathop` row also asserts
both against `mathop_expected()` in `tests/common/mod.rs` — a third, independent
transcription of the C's arithmetic (`select_operation` dispatch, the `b == 0`
guards, the priority term and the `time % 100` term). Agreement between C, Rust
and the model makes "both are identically wrong" far less likely.

## Result

**25 of 25 rows pass** across their randomized inputs, in **all four** build
configurations (`dev`/`release` x default/`--no-default-features`).
