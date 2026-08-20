# ERRORS.md — Error / rejection surface table (Phase A → Phase C)

Derived mechanically from `c_src/src/lib.c`: every `return` of an error value,
every `return NULL`, every explicit range/limit check, every loop guard that
aborts early, and every min/max constant. One row per distinct rejection.

## Constants that define the error surface

```c
#define MAX_NODES 100            /* line 36 */
#define STATUS_OK       0000     /* 0   */
#define STATUS_WARNING  0001     /* 1   (defined, never used) */
#define STATUS_ERROR    0002     /* 2   */
#define STATUS_CRITICAL 0377     /* 255 (defined, never used) */
```

**Crucial ground-truth fact:** `initialize_test_data()` (line 209) is `static`
and is referenced exactly once in the whole translation unit — by its own
definition. It is **never called**. Consequently `node_count` is permanently `0`
and `node_storage` is permanently all-zero for every caller of the public
`jumpnode()` entry point. Therefore `find_node_by_id()` *always* returns `NULL`,
and modes `0001`, `0002` and `0004` *always* take their error return. This is the
C behaviour and the Rust must reproduce it exactly — including *not* calling
`initialize_test_data`.

## Error-surface table

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `jumpnode` (`default:`, line 201) | `operation_mode` is any value other than 1, 2, 3, 4 — e.g. `0`, `5`, `-1`, `INT_MIN`, `INT_MAX`, and out-of-range "enum" ints crossing the FFI boundary | `STATUS_ERROR \| 0200` = `2 \| 128` = **130** |
| 2 | `jumpnode` case `0001` (line 124) | `find_node_by_id(node_id) == NULL`; always true because `node_count == 0`, so **every** `node_id` triggers it | `STATUS_ERROR \| 0020` = `2 \| 16` = **18** |
| 3 | `jumpnode` case `0002` (line 145) | `find_node_by_id(node_id) == NULL`; always true, so **every** `node_id` triggers it | `STATUS_ERROR \| 0040` = `2 \| 32` = **34** |
| 4 | `jumpnode` case `0004` (line 174) | `find_node_by_id(node_id) == NULL`; always true, so **every** `node_id` triggers it | `STATUS_ERROR \| 0100` = `2 \| 64` = **66** |
| 5 | `find_node_by_id` (line 52) | no stored node has `.id == id` (includes the vacuous case `node_count == 0`, i.e. always) | `NULL` — surfaced to callers as rows 2/3/4 |
| 6 | `add_node` (line 56) | `node_count >= MAX_NODES` (100) — capacity limit | `STATUS_ERROR` = **2**. Internal (`static`), unreachable from the public API because `add_node` is only called from the never-called `initialize_test_data`. Verified structurally: Rust keeps the same `>= MAX_NODES` guard returning `STATUS_ERROR`. |
| 7 | `safe_double_to_int` (line 101) | `value > 2147483647.0` → upper clamp | clamps to `2147483647.0`, returns `2147483647` |
| 8 | `safe_double_to_int` (line 104) | `value < -2147483648.0` → lower clamp | clamps to `-2147483648.0`, returns `-2147483648` |
| 9 | `safe_double_to_int` (line 108) | `value` is NaN (both comparisons false, no clamp) → `(int)NaN` | x86-64 `cvttsd2si` "integer indefinite" = `INT_MIN` = **-2147483648** |
| 10 | `jumpnode` case `0001` (line 130) | `current_node->parent_id == -1` — sentinel "no parent" terminates the walk | loop exits, `result = safe_double_to_int(accumulated_value)` |
| 11 | `jumpnode` case `0001` (line 132) | `find_node_by_id(current_node->parent_id) == NULL` — dangling parent id | `break`, then `result = safe_double_to_int(accumulated_value)` |
| 12 | `jumpnode` case `0001` (line 130) | `depth <= 0` — zero/negative depth makes the walk body execute zero times | `result = safe_double_to_int(current_node->value)` |
| 13 | `jumpnode` case `0004` (line 192) | `iter > node_storage` fails / `node_count <= 2` — backward-scan guard | `backward_sum` not added; `result` left as the sqrt accumulation |
| 14 | `jumpnode` case `0002` (line 159) | `depth` outside `[0, 16]` makes `start = array + depth` fall outside `temp_array` (`process_backward`, line 81). For `depth > 16` the `while (ptr > start)` guard is immediately false → sum `0`; for `depth < 0` the C reads out of bounds (UB) | Unreachable from the public API: row 3 returns **34** before `process_backward` is reached. Rust keeps the identical structure. |
| 15 | `jumpnode` case `0003` (line 165) | `sprintf` into `char buffer[50]`: worst case `"Node_" + 11 + "_Depth_" + 11 + NUL` = 35 bytes, so the buffer is never overrun even for `INT_MIN` | no rejection; documented boundary |
| 16 | `jumpnode` (all modes) | there are **no** null-pointer parameters — `jumpnode` takes four `int`s by value, so passing "null" is just `0`, which is row 1 (`operation_mode == 0` → 130) | **130** for `operation_mode == 0` |

### Rows that the public API cannot reach — and how they are still verified

Rows 6-14 sit behind code the public `jumpnode()` entry point provably cannot
reach, precisely *because* of rows 2/3/4 (`find_node_by_id` always returns
`NULL`). Each is verified **three** ways:

1. **Public-boundary differential test** — asserts the observable result the C
   produces at the API surface (rows 1/2/3/4 sentinels), in `tests/error_paths.rs`.
2. **Real differential test through the probe build** — `tests/deep_paths.rs`
   (feature `shadow_probe`) calls the `static` helper *itself* in both libraries
   and compares, and drives `jumpnode` with populated node storage so modes
   1/2/4 execute their real bodies. This turns rows 6-14 from "structurally
   inspected" into genuinely **executed and compared**.
3. **Structural guard assertion** — `tests/error_paths.rs` asserts the guard
   still exists verbatim in `src/lib.rs`, so the Rust cannot silently drift on a
   path a future change would expose.

Two rows have no *observable* differential trigger and are covered by (2) + (3)
only for the reasons noted:

* **Row 14, negative `depth`**: `array + start_offset` with a negative offset
  reads memory *before* `temp_array` — genuine C undefined behaviour reading
  stack garbage, with no defined result to compare against. The probe test
  therefore sweeps `start_offset` over `[0, size + 6]` and the public test pins
  the observable sentinel.
* **Rows 7/8 with `>` vs `>=`**: at exactly `2147483647.0` the clamp is a no-op,
  so that boundary mutation is semantically *equivalent* and produces no
  observable difference in either language.

## Phase C checklist

| # | public-boundary test (`tests/error_paths.rs`) | executed differential test (`tests/deep_paths.rs`) | status |
|---|---|---|---|
| 1 | `err_row01_default_mode_out_of_range_ints` | `deep_initialize_test_data_and_full_jumpnode_sweep` | [x] |
| 2 | `err_row02_mode1_node_not_found` | `deep_mode1_parent_walk_proper_trees` (miss + hit) | [x] |
| 3 | `err_row03_mode2_node_not_found` | `deep_mode2_array_backward_sum` (miss + hit) | [x] |
| 4 | `err_row04_mode4_node_not_found` | `deep_mode4_sqrt_and_backward_scan` (miss + hit) | [x] |
| 5 | `err_row05_find_node_by_id_never_matches` | `deep_add_node_and_find_node_by_id` (incl. duplicate ids → first match wins) | [x] |
| 6 | `err_row06_add_node_capacity_guard` | `deep_add_node_capacity_limit` (fills to 100, overflows by 25, asserts `STATUS_ERROR`) | [x] |
| 7 | `err_row07_safe_double_to_int_upper_clamp` | `deep_safe_double_to_int_all_shapes` (ulp-by-ulp sweep of the clamp) | [x] |
| 8 | `err_row08_safe_double_to_int_lower_clamp` | `deep_safe_double_to_int_all_shapes` | [x] |
| 9 | `err_row09_safe_double_to_int_nan` | `deep_safe_double_to_int_all_shapes` (`f64::NAN` + random NaN bit patterns) | [x] |
| 10 | `err_row10_mode1_parent_sentinel` | `deep_mode1_parent_walk_proper_trees` (root `parent_id == -1`) | [x] |
| 11 | `err_row11_mode1_dangling_parent` | `deep_mode1_dangling_parents_and_cycles` | [x] |
| 12 | `err_row12_mode1_nonpositive_depth` | `deep_mode1_parent_walk_proper_trees` (`depth <= 0`) | [x] |
| 13 | `err_row13_mode4_backward_scan_guard` | `deep_mode4_sqrt_and_backward_scan` (`node_count` 0…6 straddles `> 2`) | [x] |
| 14 | `err_row14_mode2_depth_out_of_bounds` | `deep_process_backward_offsets_and_sizes` (offsets `0..=size+6`) | [x] |
| 15 | `err_row15_mode3_sprintf_buffer_boundary` | — (reachable publicly; metric == 76 at the widest) | [x] |
| 16 | `err_row16_no_pointer_parameters_zero_is_mode0` | — | [x] |
| — | generic boundaries: `INT_MIN`/`INT_MAX`/`0`/`-1` in every argument position, out-of-range enum ints | `err_generic_boundary_matrix` | [x] |

**All 16 rows pass under all four feature combinations.**

## Mutation validation of this table

Green checkmarks only mean something if the tests can actually fail. Nine
deliberate mutations were injected into `src/lib.rs` and the suite re-run
(`src/lib.rs` was restored afterwards and byte-compared to the original):

| mutation | caught by |
|---|---|
| `compute_size_metric`: `+ 010` → `+ 011` | 15 tests, incl. all mode-3 rows |
| mode-4 sentinel `0100` → `0101` | 15 tests |
| `fmt_int`: drop the `-` sign | mode-3 rows (`C=76 Rust=72`) |
| `process_backward`: `sum +=` → `sum -=` | `deep_process_backward_offsets_and_sizes` |
| `add_node`: `data[2] = 0300` → `0301` | 7 tests |
| `find_node_by_id`: skip the first match | 9 tests |
| `MAX_NODES` 100 → 99 | `deep_add_node_capacity_limit` |
| mode-4 backward scan `i < 3` → `i < 2` | 5 tests |
| mode-4 scale `* 0.1` → `* 0.11` | 5 tests |
| mode-4 `2.718281828` → `2.718281829` (Δ = 4.9e-8) | `deep_mode4_depth_high_resolution_sweep` |

The last one initially escaped: truncation in `safe_double_to_int` hides a
sub-ulp constant error at small `depth`. `deep_mode4_depth_high_resolution_sweep`
was added to drive `depth` up to the clamp point (~1.607e8), where the
`1.0 + depth*0.1` scale amplifies the error by ~1e7 and the truncated result
moves. Only the `>` → `>=` clamp mutants remain uncaught, and those are provably
**equivalent mutants** (clamping at exactly the boundary is a no-op).
