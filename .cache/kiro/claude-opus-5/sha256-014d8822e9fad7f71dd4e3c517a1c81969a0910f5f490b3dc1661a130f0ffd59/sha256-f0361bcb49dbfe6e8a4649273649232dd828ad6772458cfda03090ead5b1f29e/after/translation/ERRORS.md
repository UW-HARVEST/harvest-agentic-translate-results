# ERRORS.md — Error-surface table

Every distinct rejection / error return / sentinel / bound in `c_src/src/lib.c`,
found by grepping for `return`, `NULL`, `STATUS_`, `>=`, `<`, `>` and the
`default:` label. One row per distinct rejection branch. Nothing invented.

`STATUS_OK 0000`=0, `STATUS_WARNING 0001`=1, `STATUS_ERROR 0002`=2,
`STATUS_CRITICAL 0377`=255. Note `STATUS_WARNING` and `STATUS_CRITICAL` are
defined but never used by any code path — recorded here for completeness.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `jumpnode` case `0001` | `find_node_by_id(node_id) == NULL`. Always true because `node_count==0` (`initialize_test_data` is never called) — so this fires for EVERY `node_id`, incl. 1..7, 0, -1, INT_MIN, INT_MAX | `return STATUS_ERROR \| 0020` = `2\|16` = **18** | [x] |
| 2 | `jumpnode` case `0002` | `find_node_by_id(node_id) == NULL` (always, same reason) | `return STATUS_ERROR \| 0040` = `2\|32` = **34** | [x] |
| 3 | `jumpnode` case `0004` | `find_node_by_id(node_id) == NULL` (always, same reason) | `return STATUS_ERROR \| 0100` = `2\|64` = **66** | [x] |
| 4 | `jumpnode` `default:` | `operation_mode` not in {1,2,3,4} — i.e. out-of-range "enum" value crossing FFI: `0`, `5`, `6`, `8`, `-1`, `INT_MIN`, `INT_MAX`, `0x10001`, and every other int | `result = STATUS_ERROR \| 0200` = `2\|128` = **130** | [x] |
| 5 | `find_node_by_id` | id not present in `node_storage[0..node_count]` (loop falls through) | `return NULL` (sentinel consumed by rows 1–3) | [x] |
| 6 | `add_node` | `node_count >= MAX_NODES` (100) | `return STATUS_ERROR` = **2**; internal linkage, unreachable from the ABI because its only caller `initialize_test_data` is never called | [x] |
| 7 | `safe_double_to_int` | `value > 2147483647.0` → clamped | returns **2147483647** | [x] |
| 8 | `safe_double_to_int` | `value < -2147483648.0` → clamped | returns **-2147483648** | [x] |
| 9 | `safe_double_to_int` | `value` is NaN (neither clamp fires; `(int)` cast is UB in C, x86-64 `cvttsd2si` yields the integer-indefinite value) | returns **INT_MIN** = -2147483648 | [x] |
| 10 | `process_backward` | `start_offset >= size` (i.e. `depth >= 16`) → `ptr > start` false on entry, loop body never runs | `return 0` (sum untouched) | [x] |
| 11 | `process_backward` | `start_offset < 0` (`depth < 0`) → `start` is before the array, loop reads `array[-1]`, `array[-2]`, … out of bounds | undefined / reads stack garbage. **Unreachable from the ABI**: case `0002` returns row 2's error before `process_backward` is ever called | [x] |
| 12 | `jumpnode` case `0001` inner loop | `find_node_by_id(current_node->parent_id) == NULL` → `break` (no `i++`) | loop exits early, `result` from partial accumulation. Unreachable (row 1 fires first) | [x] |
| 13 | `jumpnode` case `0001` inner loop | `current_node->parent_id == -1` → loop guard false | loop does not iterate. Unreachable (row 1 fires first) | [x] |
| 14 | `jumpnode` case `0004` | `node_count > 2` false (it is 0) → backward-sum block skipped entirely | no `backward_sum` added. Unreachable (row 3 fires first) | [x] |
| 15 | `jumpnode` case `0004` | `iter > node_storage` false → backward walk stops before 3 iterations | partial `backward_sum`. Unreachable (row 3 fires first) | [x] |
| 16 | `compute_size_metric` | `str` such that `strlen` is 0 (empty string) | `metric = 0*2 + 010` = **8**. Not reachable via `jumpnode`: `sprintf` always writes at least `"Node_0_Depth_0"` (14 chars) | [x] |
| 17 | `jumpnode` case `0003` | `flags` negative / with high bits set → masked by `& 0177` | only the low 7 bits contribute; e.g. `flags=-1` adds `127` | [x] |
| 18 | `jumpnode` case `0003` | `node_id`/`depth` = `INT_MIN` → `%d` prints `-2147483648` (11 chars), longest possible `buffer` content (5+11+7+11=34 < 50) | no overflow; `metric = 34*2+8` = `76` plus mask | [x] |

## Generic FFI boundaries also covered (not distinct C branches)

| trigger | expectation |
|---|---|
| all four args `0` | mode 0 → `default:` → 130 |
| all four args `INT_MIN` | mode INT_MIN → `default:` → 130 |
| all four args `INT_MAX` | mode INT_MAX → `default:` → 130 |
| out-of-range enum values for `operation_mode`: every int in `-1024..1024` plus INT_MIN/INT_MAX/±2^k | 130 for all except 1,2,3,4 |
| `node_id`/`depth` at `INT_MIN`, `INT_MAX`, `0`, `-1`, `±10^k`, `±(10^k-1)` (digit-count boundaries of `%d`) | see CONFIGS rows 6–13 |
| `flags` at `0`, `-1`, `INT_MIN`, `INT_MAX`, `0177`, `0200`, `0377` | mode 3: `+ (flags & 0177)` |

There are no pointer parameters in this ABI (`jumpnode` takes four `int`s),
so null-pointer and length arguments are not applicable. Rows 7–11 and 16
are exercised through the internal paths that reach them where reachable and
recorded as ABI-unreachable where the C returns earlier; rows 1–4, 17, 18
are directly reachable and each has a differential test.

## How each row is covered

Rows 1–5, 17, 18 and the generic-boundary table are directly reachable through
`jumpnode` and are tested in `tests/phase_c_error_paths.rs` (13 tests). Each
asserts the SAME error code on both sides *and* pins that code to the C's
literal ground-truth value (18 / 34 / 66 / 130), so "both failed somehow" is not
accepted.

Rows 6–16 sit behind `static` functions that `jumpnode` can never reach
(`node_count` is pinned at 0). They are covered in
`tests/phase_b_internals.rs`, which reaches the C `static`s through a probe
`.so` compiled from `tests/c_probe/probe.c` — that file `#include`s the
**unmodified** `c_src/src/lib.c` and re-exports its internals. `c_src/` is not
touched, and the CMake-built `.so` still exports only `jumpnode`.

| ERRORS.md row | covering test |
|---|---|
| 1 | `err01_mode1_node_not_found_returns_18` |
| 2 | `err02_mode2_node_not_found_returns_34` |
| 3 | `err03_mode4_node_not_found_returns_66` |
| 4 | `err04_default_arm_out_of_range_enum_returns_130` |
| 5 | `err05_find_node_by_id_null_sentinel_consistent_across_arms`, `internals_find_node_by_id_matches` |
| 6 | `internals_add_node_max_nodes_boundary`, `internals_add_node_fills_storage_identically` |
| 7, 8, 9 | `internals_safe_double_to_int_matches_including_clamps_and_nan` (200 000 random f64 **bit patterns**, so NaNs/±inf/subnormals are hit; plus every integer straddling both clamps) |
| 10 | `internals_process_backward_matches` (`start_offset` swept 0..=24 against `size` 0..=20) |
| 11 | not differentially testable: negative `start_offset` reads before the array, which is undefined in the C original and returns whatever stack bytes happen to be there. Confirmed ABI-unreachable instead (`err10_11_...`), and the Rust reproduces the same pointer arithmetic (`wrapping_offset`) rather than panicking. |
| 12, 13 | `internals_jumpnode_arms_with_populated_storage` scenarios 2/3/5 (root with `parent_id == -1`, a dangling `parent_id`, and a parent **cycle**) |
| 14, 15 | `internals_jumpnode_arms_with_populated_storage` scenarios 1/2/3 (`node_count` = 0, 1, 2 — both sides of the `node_count > 2` guard and the `iter > node_storage` stop) |
| 16 | `internals_compute_size_metric_matches` (length 0 included) |
| 17 | `err17_mode3_flag_mask_discards_high_bits` |
| 18 | `err18_mode3_int_min_widest_format_no_overflow`, `internals_sprintf_node_depth_bytes_match` |

Every row is checked off. Sensitivity was verified by mutation testing — see the
harness-validation section of `CONFIGS.md`; 31/31 injected single-edit bugs,
including one in each error constant, were caught.
