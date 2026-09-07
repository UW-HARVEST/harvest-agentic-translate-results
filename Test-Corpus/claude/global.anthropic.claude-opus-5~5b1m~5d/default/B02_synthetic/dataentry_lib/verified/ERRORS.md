# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. Every `return -1` / `return -2` /
`return NULL` / `return 0` rejection branch, every explicit range/NULL check and
every min/max constant in the C source gets one row. There are **no** `assert`s
and **no** error enums in this library; the only channel is the `int` return
value of `dataentry` (sentinels `-1`, `-2`, `0`).

Reachability legend:
* **live** — reachable through the public `dataentry` ABI.
* **dead** — the guard exists in the C source but is provably unreachable given
  the values `dataentry` can pass. The Rust must still reproduce the guard so
  the two implementations stay behaviourally identical; the row is verified by
  asserting the *observable* result of the surrounding path matches.

## Constants / limits

| const | value | where | note |
|---|---|---|---|
| `MAX_ENTRIES` | 10 | `#define`, line 28 | **never referenced** by any C code — imposes no limit; `count` is unbounded |
| `NAME_LENGTH` | 32 | `#define`, line 29 | `DataEntry.name[32]`, `char buffer[32]`, `char temp_name[32]`, `max_len` argument |
| `lookup_table` | 4 rows x 3 cols | line 37 | bounds enforced only by the caller-side check in `dataentry` case 3 |
| `sizeof(DataEntry)` | 40 | 4 + 4 + 32 | multiplier in `count * sizeof(DataEntry)` |

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | reach |
|---|----------|----------------------------------------------|-------------------|-------|
| 1 | `find_entry` (lib.c:55) | loop runs to `end` without `ptr->id == target_id` | returns `NULL` | live |
| 2 | `find_entry` (lib.c:46-48) | `count <= 0` → `end <= ptr`, loop body never entered | returns `NULL` immediately | live (via #1's callers) |
| 3 | `process_name` (lib.c:61) | `dest == NULL` | returns `-1`, no `strcpy` | dead (`dataentry` always passes `&buffer[0]`) |
| 4 | `process_name` (lib.c:61) | `*dest == '\0'` (non-NULL but empty dest) | returns `-1`, no `strcpy` | dead (`buffer` holds `"Default"` at the call) |
| 5 | `process_name` (lib.c:65) | `src` longer than `NAME_LENGTH` — `max_len` is **ignored**, `strcpy` is unbounded | no rejection: overflows `dest` | dead (`src` is the 8-byte literal `"TestName"`) |
| 6 | `calculate_lookup` (lib.c:79) | `lookup_table[row][col] == 0` | returns `0`, `*result` left untouched | dead (no zero entry in the table) |
| 7 | `calculate_lookup` (lib.c:74) | `row`/`col` out of `[0,4)` / `[0,3)` | out-of-bounds read (UB) | unreachable — guarded by row #14-17 |
| 8 | `create_entries` (lib.c:89) | `malloc` returned `NULL` (huge `count * 40`, e.g. `count == INT_MAX`) | returns `NULL` | live |
| 9 | `create_entries` (lib.c:89) | `count <= 0` (malloc already happened → leaked) | returns `NULL` | dead via `dataentry` (`count = p1 > 0 ? p1 : 5/3`), asserted structurally |
| 10 | `modify_entries` (lib.c:111) | `entries == NULL` | returns `-1` (indistinguishable from a real total of `-1`) | dead (`dataentry` checks `entries` first) |
| 11 | `modify_entries` (lib.c:118) | `count <= 0` → `last <= current`, no iteration | returns `0` → `dataentry` case 2 leaves `result == 0` | dead via `dataentry` |
| 12 | `modify_entries` (lib.c:119) | `current->value == 0` | that entry is skipped, not multiplied, not summed | dead (`(200+i)*10 != 0` for `i >= 0`) |
| 13 | `dataentry` case 1 (lib.c:146) | `create_entries(count,100) == NULL` (i.e. `param1` so large that `malloc` fails) | `result = -1`; `free` **not** called | live |
| 14 | `dataentry` case 1 (lib.c:146) | `count == 0` | `result = -1` | dead (`count >= 1` always) |
| 15 | `dataentry` case 1 (lib.c:153) | `find_entry` returned `NULL`, i.e. `param2 < 0` or `param2 >= count` (no id `100+param2`) | `result = -2` | live |
| 16 | `dataentry` case 1 (lib.c:153) | `found->id == 0` | `result = -2` | dead (ids are `100..100+count-1`) |
| 17 | `dataentry` case 1 (lib.c:151) | `100 + param2` overflows `int` (`param2 == INT_MAX`) | wraps; no id matches → `result = -2` | live |
| 18 | `dataentry` case 2 (lib.c:168) | `create_entries(count,200) == NULL` (`malloc` failure) | `result = -1`; `free` **not** called | live |
| 19 | `dataentry` case 2 (lib.c:173) | `modify_entries(...) == 0` (e.g. `param2 == 0` → every value becomes 0, total 0) | `result` stays `0`; `param3` is **not** added | live |
| 20 | `dataentry` case 2 (lib.c:173) | total happens to wrap to exactly `0` | `param3` not added | live |
| 21 | `dataentry` case 3 (lib.c:181) | `param1 < 0` | guard fails → `result` stays `0` | live |
| 22 | `dataentry` case 3 (lib.c:181) | `param1 >= 4` (one past the 4-row table, incl. `INT_MAX`) | `result` stays `0` | live |
| 23 | `dataentry` case 3 (lib.c:181) | `param2 < 0` (incl. `INT_MIN`) | `result` stays `0` | live |
| 24 | `dataentry` case 3 (lib.c:181) | `param2 >= 3` (one past the 3-col table) | `result` stays `0` | live |
| 25 | `dataentry` case 3 (lib.c:182) | `calculate_lookup` returned `0` | `result` stays `0`, `param3` not added, `lookup_result` uninitialised | dead (row #6) |
| 26 | `dataentry` default (lib.c:190) | `process_name` returned `-1` | `result = -1` before being overwritten at line 193 | dead (row #3/#4) |
| 27 | `dataentry` default (lib.c:192) | `strlen(buffer) == 0` | `result` keeps `process_name`'s return value | dead (`buffer == "TestName"`, len 8) |
| 28 | `dataentry` dispatch (lib.c:141) | `mode` matches no `case` — i.e. any `int` other than 1/2/3 (`0`, negative, `4`, `INT_MIN`, `INT_MAX`, out-of-range "enum" values) | falls to `default:` → returns `8 * param1` | live |

### Additional generic FFI boundaries covered by the tests

These are not source-level `return` sites but are the standard boundaries the
task requires to be probed through the FFI wall. `dataentry` takes only scalars,
so there is no pointer parameter to null; the equivalent boundaries are:

| # | boundary | expected |
|---|---|---|
| G1 | zero for every parameter (`mode/param1/param2/param3 == 0`) | must agree |
| G2 | `INT_MIN` / `INT_MAX` for every parameter, one at a time and combined | must agree |
| G3 | one step past each documented valid range: `mode` = `0` and `4`; `param1` = `-1`/`4` for case 3; `param2` = `-1`/`3` for case 3; `param2` = `-1`/`count` for case 1 | must agree |
| G4 | out-of-range "enum" `mode` values (C `switch` on `int` accepts anything) | must take `default:` identically |
| G5 | oversized length: `param1` = `INT_MAX`, `1<<28`, `1<<24` for cases 1/2 (drives `malloc` size `count*40` past `SIZE_MAX`/RAM) | must agree on the `-1` vs. success outcome |

## Result

All 28 table rows plus G1-G5 have a passing differential test in
`tests/phase_c_errors.rs` (20 test functions; several rows share one test where
they are triggered by the same construction). The malloc-failure rows (8, 13,
18, G5) are exercised inside a child process launched under
`sh -c 'ulimit -v 524288'`, because this host has 371 GiB of RAM and heuristic
overcommit, so an 80 GiB `malloc` would otherwise SUCCEED and the fill loop
would touch all of it. Under the address-space limit all 10 probed sizes fail to
allocate and both libraries return `-1` identically.

Reachability claims in the table are not taken on faith: each "dead" row has a
test that asserts the *observable consequence* of the branch being unreachable
(for example, `err09_14` asserts mode 1 never returns `-1` for a non-huge
`param1`, and `err16` asserts a `find_entry` hit always yields the entry's
value rather than `-2`).
