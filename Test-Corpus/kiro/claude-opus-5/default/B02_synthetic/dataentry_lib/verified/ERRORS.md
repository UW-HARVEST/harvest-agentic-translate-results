# ERRORS.md — error-surface table

Mechanically derived from `c_src/src/lib.c`. Every `return -1`, `return NULL`,
`return 0`-as-failure, explicit range check, null check, and size constant is
listed. There are no `assert`s and no error enums in this library
(`grep -n 'assert\|errno\|enum' c_src/src/lib.c` → no matches).

Size/limit constants found: `MAX_ENTRIES 10` (defined, never referenced),
`NAME_LENGTH 32`, `lookup_table` dimensions `[4][3]`.

`dataentry` is the ONLY externally reachable entry point, so each row records
both the internal trigger and the value observable at the FFI boundary.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| 1  | `create_entries` | `entries == NULL` (malloc failed) — `count * sizeof(DataEntry)` too large for the allocator | returns `NULL`; caller (`dataentry` case 1/2) sets `result = -1` |
| 2  | `create_entries` | `count <= 0` after a successful malloc (allocation is leaked, still rejected) | returns `NULL` → `result = -1` |
| 3  | `find_entry` | `count <= 0` → `ptr < end` false on first test, loop body never runs | returns `NULL` → case 1 `result = -2` |
| 4  | `find_entry` | no element has `id == target_id` (loop exhausts) | returns `NULL` → case 1 `result = -2` |
| 5  | `process_name` | `dest == NULL` | returns `-1` |
| 6  | `process_name` | `*dest == '\0'` (first byte of dest is NUL) | returns `-1` |
| 7  | `calculate_lookup` | `lookup_table[row][col] == 0` → falls through `if` | returns `0` → case 3 leaves `result = 0` |
| 8  | `modify_entries` | `entries == NULL` | returns `-1` |
| 9  | `dataentry` case 1 | `entries == NULL` (row 1/2) `|| count == 0` | `-1` |
| 10 | `dataentry` case 1 | `found == NULL` (row 3/4): `param2 < 0` | `-2` |
| 11 | `dataentry` case 1 | `found == NULL` (row 3/4): `param2 >= count`, i.e. one past the last valid index | `-2` |
| 12 | `dataentry` case 1 | `found->id == 0` (dead branch: ids are `100 + i`, never 0) | `-2` if reachable |
| 13 | `dataentry` case 2 | `entries == NULL` (row 1/2) | `-1` |
| 14 | `dataentry` case 2 | `modify_entries` result is `0` → `if ((result = ...))` false, `param3` NOT added | `0` (not `param3`) |
| 15 | `dataentry` case 3 | `param1 < 0` — row index below range | `0` (switch case body skipped) |
| 16 | `dataentry` case 3 | `param1 >= 4` — row index at/past `lookup_table` row count | `0` |
| 17 | `dataentry` case 3 | `param2 < 0` — col index below range | `0` |
| 18 | `dataentry` case 3 | `param2 >= 3` — col index at/past `lookup_table` col count | `0` |
| 19 | `dataentry` case 3 | `param1 == INT_MIN` / `INT_MAX`, `param2 == INT_MIN` / `INT_MAX` (extreme out-of-range indices) | `0` |
| 20 | `dataentry` default | `mode` has no matching case: `0`, negative, `> 3`, `INT_MIN`, `INT_MAX` — out-of-range "enum" values crossing FFI | `8 * param1` (`strlen("TestName") == 8`) |
| 21 | `dataentry` default | `if ((count = strlen(buffer)))` false (dead branch: buffer holds `"TestName"`, len 8) | `result` would stay `8` |
| 22 | `dataentry` case 1 | `param1 <= 0` → `count` defaults to `5`; `param2` outside `[0,5)` | `-2` |
| 23 | `dataentry` case 2 | `param1 <= 0` → `count` defaults to `3` | sum over 3 entries |

## Rows unreachable from the FFI boundary

Rows 5, 8, 12, 21 are dead code in the C as written (`process_name` is only
called with a live buffer whose first byte is `'D'`; `modify_entries` is only
called after a NULL check; entry ids are `100 + i`; `buffer` always holds a
non-empty string). They are recorded for completeness and are asserted at the
observable level: the paths that would reach them return the values in rows
20/14 instead. Row 7 is likewise unreachable because every
`lookup_table[row][col]` element is non-zero.

Rows 1 and 2 are reachable: `count <= 0` is unreachable through `dataentry`
(both cases coerce to `param1 > 0 ? param1 : 5|3`), but malloc failure is
reachable with a huge `param1` (`param1 * 40` bytes).

## Generic FFI boundaries also covered by the tests

- Out-of-range "enum" values for `mode` (any `int` is a legal argument): row 20.
- Zero and negative lengths/counts: `param1 == 0`, `param1 < 0` → rows 22, 23.
- Oversized lengths: `param1` near `INT_MAX` → row 1.
- One step past a documented valid range: `param1 == 4`, `param2 == 3` in
  case 3 (rows 16, 18); `param2 == count` in case 1 (row 11).
- `INT_MIN` / `INT_MAX` for all four parameters: row 19 and the sweep tests.
- Null pointers: the public ABI takes no pointer arguments, so there is no
  null-pointer input to pass. The internal null checks are rows 1, 5, 8.
