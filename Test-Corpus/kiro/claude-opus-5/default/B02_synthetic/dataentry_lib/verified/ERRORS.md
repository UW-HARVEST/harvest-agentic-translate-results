# ERRORS.md — error-surface table

Every distinct rejection/error path in `c_src/src/lib.c`, derived mechanically
by grepping for `return NULL`, `return -1`, `return 0`, `assert`, every explicit
range/null/zero check, and every min/max constant. Line numbers refer to
`c_src/src/lib.c`.

Constants that bound the input space:

| constant | value | where it constrains |
|---|---|---|
| `MAX_ENTRIES` | 10 | **declared but never referenced** — imposes no check |
| `NAME_LENGTH` | 32 | `DataEntry.name` size, `buffer` size, `process_name` `max_len` arg (ignored) |
| `lookup_table` rows | 4 | `mode 3`: `param1 >= 0 && param1 < 4` |
| `lookup_table` cols | 3 | `mode 3`: `param2 >= 0 && param2 < 3` |

The only observable channel is the `int` return value of `dataentry`, so
"expected C result" is stated as that return value (internal helper returns are
noted where they are the proximate cause).

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `find_entry` (L55) | loop runs to `end` with no `ptr->id == target_id` | returns `NULL` → surfaces as `dataentry` mode 1 result `-2` |
| 2 | `find_entry` (L47) | `count <= 0` so `ptr < end` is false on entry | returns `NULL` (loop body never runs) → mode 1 result `-2` |
| 3 | `process_name` (L61-62) | `dest == NULL` | returns `-1` |
| 4 | `process_name` (L61-62) | `*dest == '\0'` (dest points at an empty string) | returns `-1` |
| 5 | `calculate_lookup` (L79) | `lookup_table[row][col] == 0` | returns `0`; **unreachable** — all 12 cells are 10..120, nonzero |
| 6 | `create_entries` (L89-90) | `malloc` returned `NULL` (allocation too large / OOM) | returns `NULL` |
| 7 | `create_entries` (L89-90) | `count <= 0` (allocation already made and leaked) | returns `NULL` |
| 8 | `dataentry` mode 1 (L146-147) | `create_entries` returned `NULL` (i.e. `malloc` failed for `count*40` bytes) | `dataentry` returns `-1` |
| 9 | `dataentry` mode 1 (L146-147) | `count == 0` | **unreachable**: `count = param1 > 0 ? param1 : 5` is never 0 |
| 10 | `dataentry` mode 1 (L153-154) | `find_entry` returned `NULL`, i.e. `100 + param2` is not in `[100, 100+count)`; in particular `param2 < 0` or `param2 >= count` | returns `-2` |
| 11 | `dataentry` mode 1 (L153-154) | `found->id == 0` | **unreachable** for reachable `count`: ids are `100 + i`, `i in [0,count)` |
| 12 | `dataentry` mode 2 (L168-169) | `create_entries` returned `NULL` (`malloc` failed for `count*40`) | returns `-1` |
| 13 | `dataentry` mode 2 (L172) | `modify_entries` total is `0` → the `if` is false, `param3` is **not** added | returns `0`. Triggered by `param2 == 0` (every `value*0 == 0`) or by a total that wraps to exactly 0 |
| 14 | `modify_entries` (L111-112) | `entries == NULL` | returns `-1`; unreachable from `dataentry` (both call sites null-check first) |
| 15 | `dataentry` mode 3 (L181) | `param1 < 0` | range check fails, `result` stays `0` → returns `0` |
| 16 | `dataentry` mode 3 (L181) | `param1 >= 4` (one past the 4-row bound) | returns `0` |
| 17 | `dataentry` mode 3 (L181) | `param2 < 0` | returns `0` |
| 18 | `dataentry` mode 3 (L181) | `param2 >= 3` (one past the 3-col bound) | returns `0` |
| 19 | `dataentry` `default` (L188) | `mode` is any value other than 1, 2, 3 — including `0`, negatives, `INT_MIN`, `INT_MAX`, and out-of-range "enum-like" ints | falls into `default`: returns `8 * param1` |
| 20 | `dataentry` `default` (L191) | `strlen(buffer) == 0` after `process_name` | **unreachable**: `buffer` holds `"TestName"`, length 8 → the `-1`/`8` from `process_name` is always overwritten by `8 * param1` |

## Notes on unreachable rows

Rows 5, 9, 11, 14 and 20 are genuine branches in the C source that cannot be
reached through the public `dataentry` entry point. They are still listed (the
table is derived from what the C *checks*, not from what is reachable) and are
still tested: the differential tests drive the inputs that would reach them if
they were reachable, and assert C and Rust agree on the observed result.

## Generic FFI boundary cases also covered in Phase C

* `mode` values with no matching `case` label (out-of-range enum-like ints):
  `0, -1, 4, 5, 100, INT_MIN, INT_MAX`.
* Zero and negative "length" style parameters: `param1 = 0`, `param1 < 0`,
  `param1 = INT_MIN` for every mode.
* One step past every documented valid range: `param1 = -1/4`,
  `param2 = -1/3` for mode 3; `param2 = -1/count` for mode 1.
* Oversized lengths for the allocating modes: `param1` large enough that
  `count * sizeof(DataEntry)` cannot be allocated (`INT_MAX`, `INT_MAX/2`,
  `0x4000_0000`) — exercises rows 6/8/12.
* There are no pointer parameters on the public API, so null-pointer inputs are
  only reachable internally (rows 3, 4, 14) and are covered by argument-driven
  proxies plus the unreachability argument above.

## Row -> test mapping and status

Every row has a passing differential test in `tests/phase_c_errors.rs`. Each test
asserts (a) that C and Rust return the same value, and (b) that the value is the
specific sentinel the row predicts — not merely that "both failed".

| rows | test | status |
|---|---|---|
| 1, 2, 10 | `row01_02_10_find_entry_null_yields_minus2` — asserts the `-2` sentinel for 20k+ randomized misses on both sides of the valid index window | [x] |
| 3, 4 | `row03_04_process_name_guard_unreachable` — pins that `process_name`'s `-1` can never escape `dataentry`; asserts the default arm always yields `8 * param1`, including `param1 == 0` where a leaked `-1` would be visible | [x] |
| 5 | `row05_calculate_lookup_zero_cell_unreachable` — asserts the exact `cell * 2 + param3` value for all 12 cells, so a zero-cell (`result` stays 0) path is excluded | [x] |
| 6, 8 | `row06_08_mode1_alloc_failure_minus1` — asserts `-1` for `INT_MAX`, `INT_MAX-1`, `INT_MAX/2`, `0x40000000`, `0x20000000` | [x] |
| 6, 12 | `row06_12_mode2_alloc_failure_minus1` — same counts on mode 2, crossed with several multipliers | [x] |
| 7, 9 | `row07_09_nonpositive_count_unreachable` — asserts non-positive `param1` never produces the NULL-alloc sentinel, and that C and Rust agree | [x] |
| 13 | `row13_mode2_zero_total_skips_param3` — asserts exactly `0` (param3 NOT added) for `param2 == 0` across counts and addends incl. `INT_MAX`/`INT_MIN` | [x] |
| 14 | `row14_modify_entries_null_guard_unreachable` — 20k randomized mode-2 calls agree | [x] |
| 15, 16, 17, 18 | `row15_18_mode3_range_rejections` — asserts `0` for every one-step-past bound on both axes, plus a 20k randomized full-`i32` sweep that asserts `0` exactly when the guard fails | [x] |
| 19, 20 | `row19_20_out_of_range_mode_values` — out-of-range enum-like `mode` ints (`0, 4..10, -1, -2, -3, -100, 1000, INT_MIN, INT_MIN+1, INT_MAX, INT_MAX-1, ±0x10000`) all land in `default` and yield `8 * param1` | [x] |
| generic | `generic_zero_and_extreme_lengths` — full `{0, 1, -1, INT_MIN, INT_MAX}` cross-product on all four arguments for 8 modes | [x] |
| generic | `generic_one_past_every_bound` — one step past the lookup-table bounds, the `MAX_ENTRIES` neighbourhood, the `NAME_LENGTH` neighbourhood, and the `switch` labels | [x] |

**All 20 rows checked. 12 error-path tests, passing in all four build
configurations.**

### Null pointers

The public API takes no pointer arguments (`int, int, int, int`), so a caller
cannot pass a null pointer across the FFI boundary. The three internal null
checks (rows 3, 4, 14) are covered by the unreachability tests above, which pin
the observable consequence of the guards never firing.
