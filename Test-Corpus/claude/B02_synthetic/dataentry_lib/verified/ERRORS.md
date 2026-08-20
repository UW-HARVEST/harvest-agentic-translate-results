# ERRORS.md — Phase C error-surface table

Mechanically grepped from `c_src/src/lib.c`: every `return NULL`, `return -1`,
`return 0` taken as a rejection, every `result = -N` assignment, every explicit
range/null check, and every constant bound (`MAX_ENTRIES`, `NAME_LENGTH`,
`lookup_table` dimensions `4`/`3`).

The only exported symbol is `int dataentry(int, int, int, int)`; all other
functions are `static` (internal linkage). Rows whose trigger lives in a
`static` helper are marked with their reachability through the public ABI —
those that cannot be constructed from outside are noted as
**unreachable-by-construction** and are verified by inspection plus a
differential test of the enclosing public path.

`dataentry` takes no pointers, so there is no null-pointer argument surface on
the public ABI; `mode` is the enum-like selector and out-of-range `mode` values
(any `int` with no valid variant) are covered by row 18.

| #  | function | trigger (exact invalid input/condition) | expected C result | test |
|----|----------|------------------------------------------|-------------------|------|
| 1  | `process_name` (lib.c:61) | `dest == NULL` | returns `-1` | unreachable-by-construction: sole call site passes stack `buffer`. Covered by `err_01_02_process_name_guards` (default branch) |
| 2  | `process_name` (lib.c:61) | `*dest == '\0'` (dest non-null, empty string) | returns `-1` | unreachable-by-construction: `strcpy(buffer,"Default")` runs first, so `buffer[0]=='D'`. Covered by `err_01_02_process_name_guards` |
| 3  | `find_entry` (lib.c:55) | loop reaches `end` without `ptr->id == target_id` | returns `NULL` | `err_03_09_find_entry_miss` |
| 4  | `create_entries` (lib.c:89) | `malloc(count * sizeof(DataEntry))` returns `NULL` (huge `count`) | returns `NULL` | `err_04_08_10_malloc_failure` |
| 5  | `create_entries` (lib.c:89) | `count <= 0` | returns `NULL` | unreachable-by-construction: `count = param1 > 0 ? param1 : 5/3` is always `> 0`. Covered by `err_05_count_never_nonpositive` |
| 6  | `modify_entries` (lib.c:111) | `entries == NULL` | returns `-1` | unreachable-by-construction: mode 2 returns early on `entries == NULL` before calling. Covered by `err_06_modify_entries_null_guard` |
| 7  | `calculate_lookup` (lib.c:79) | `lookup_table[row][col] == 0` | returns `0` (leaves `*result` untouched) | unreachable-by-construction: no zero in the table. Covered by `err_07_lookup_never_zero` (all 12 cells) |
| 8  | `dataentry` mode 1 (lib.c:146) | `entries == NULL` (allocation failed) | `result = -1` | `err_04_08_10_malloc_failure` |
| 9  | `dataentry` mode 1 (lib.c:153) | `found == NULL` — `100 + param2` matches no id | `result = -2` | `err_03_09_find_entry_miss` |
| 10 | `dataentry` mode 1 (lib.c:146) | `count == 0` | `result = -1` | unreachable-by-construction (see row 5); `err_05_count_never_nonpositive` asserts `param1 == 0` takes the `count = 5` path in both |
| 11 | `dataentry` mode 1 (lib.c:153) | `found->id == 0` | `result = -2` | unreachable-by-construction: ids start at `base_id = 100`. Covered by `err_11_found_id_never_zero` |
| 12 | `dataentry` mode 2 (lib.c:168) | `entries == NULL` (allocation failed) | `result = -1` | `err_04_08_10_malloc_failure` |
| 13 | `dataentry` mode 3 (lib.c:181) | `param1 < 0` (row underflow, incl. `INT_MIN`) | `result = 0` (no lookup) | `err_13_16_mode3_range_rejects` |
| 14 | `dataentry` mode 3 (lib.c:181) | `param1 >= 4` (row overflow, incl. `4` and `INT_MAX`) | `result = 0` | `err_13_16_mode3_range_rejects` |
| 15 | `dataentry` mode 3 (lib.c:181) | `param2 < 0` (col underflow, incl. `INT_MIN`) | `result = 0` | `err_13_16_mode3_range_rejects` |
| 16 | `dataentry` mode 3 (lib.c:181) | `param2 >= 3` (col overflow, incl. `3` and `INT_MAX`) | `result = 0` | `err_13_16_mode3_range_rejects` |
| 17 | `dataentry` mode 2 (lib.c:173) | `modify_entries` returns `0` (`param2 == 0` multiplier) → the `if` is false | `result = 0`, `param3` NOT added | `err_17_mode2_zero_total_skips_param3` |
| 18 | `dataentry` (lib.c:188) | `mode` not in `{1,2,3}` — out-of-range enum value across FFI (`0`, `-1`, `4`, `5`, `INT_MIN`, `INT_MAX`, random) | falls into `default:` → `result = strlen("TestName") * param1 = 8 * param1` | `err_18_out_of_range_mode` |

## Additional generic boundaries covered (not distinct C branches)

| # | condition | note | test |
|---|-----------|------|------|
| G1 | `param1 == 0` / negative in modes 1 & 2 | selects the `5` / `3` default count, never a zero-size allocation | `err_05_count_never_nonpositive` |
| G2 | oversized length: `param1 = INT_MAX` (mode 1 & 2) → `malloc(85899345880)` fails | `-1` | `err_04_08_10_malloc_failure` |
| G3 | one step past valid index (mode 1): `param2 == count` | `-2` | `err_03_09_find_entry_miss` |
| G4 | one step past valid range (mode 3): `param1 == 4`, `param2 == 3` | `0` | `err_13_16_mode3_range_rejects` |
| G5 | signed wraparound of `100 + param2` (mode 1) at `INT_MAX`/`INT_MIN` | wraps; no match | `err_g5_param2_wraparound` |
| G6 | signed wraparound of `total + param3` (mode 2) and `8 * param1` (default) | wraps | `cfg_*` rows in CONFIGS.md |
| G7 | `NAME_LENGTH` (32) bound on `sprintf`/`strcpy` of `"Entry_%d"` | max 17 chars + NUL, never overflows | `cfg_*` mode-1 rows (the name is copied into `buffer`) |
| G8 | `MAX_ENTRIES` (10) | defined but never referenced by any C code path — no check exists | n/a (documented dead constant) |

## Status

All 18 rows and generic boundaries G1–G7 have a passing differential test in
`tests/phase_c_errors.rs` (12 tests, every one asserting the *same specific*
sentinel from both `.so`s, not merely "both failed"). G8 is a dead constant that
no C code path ever checks, so there is no behavior to compare.

```
cargo build --release && cargo test --release --test phase_c_errors
test result: ok. 12 passed; 0 failed
```

Sentinel coverage actually exercised across both implementations: `-1`
(allocation failure, rows 4/8/12), `-2` (lookup miss, rows 3/9), `0` (mode-3
range rejection rows 13–16 and the zero-total row 17), and the `default:`
fall-through for out-of-range `mode` (row 18). The rows marked
*unreachable-by-construction* (1, 2, 5, 6, 7, 10, 11) are proven unreachable
from the exported ABI in the comment above each test, and each has a test that
pins the observable consequence in both implementations instead of faking the
condition.

Re-verified with the C oracle rebuilt at `-O0`, `-O2` and `-O3`, in both the
debug and release Rust profiles, and under every Cargo feature combination.
