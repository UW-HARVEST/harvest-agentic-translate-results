# ERRORS.md — Error / rejection surface table (Phase A → gated by Phase C)

Mechanically derived from `c_src/src/lib.c`. Every `return NULL`, `return -1`,
`return 0` error branch, every explicit range check, every null check, and every
min/max constant gets **one row**. There are no `assert`s in the C source.

Grep basis:

```
c_src/src/lib.c:55   return NULL;                       # find_entry: no match
c_src/src/lib.c:61      if (dest == NULL || *dest == '\0')  # process_name guard
c_src/src/lib.c:62      return -1;
c_src/src/lib.c:79   return 0;                          # calculate_lookup: table cell == 0
c_src/src/lib.c:89      if (entries == NULL || count <= 0)  # create_entries guard
c_src/src/lib.c:90      return NULL;
c_src/src/lib.c:111     if (entries == NULL)               # modify_entries guard
c_src/src/lib.c:112     return -1;
c_src/src/lib.c:146     if (entries == NULL || count == 0) -> result = -1   # mode 1
c_src/src/lib.c:153     if (found == NULL || found->id == 0) -> result = -2 # mode 1
c_src/src/lib.c:168     if (entries == NULL) -> result = -1                 # mode 2
c_src/src/lib.c:173     if ((result = modify_entries(...)))  # falsy -> skip += param3
c_src/src/lib.c:181     if (param1 >= 0 && param1 < 4 && param2 >= 0 && param2 < 3)  # range check
c_src/src/lib.c:182     if ((result = calculate_lookup(...)))               # falsy -> result stays 0
c_src/src/lib.c:192     if ((count = strlen(buffer)))       # falsy -> result keeps process_name rc
c_src/src/lib.c:28   #define MAX_ENTRIES 10     # dead constant, never checked
c_src/src/lib.c:29   #define NAME_LENGTH 32     # buffer/name width, passed as unused max_len
```

Only `dataentry` is reachable across the FFI boundary, so each row states the
observable `dataentry` return value that the branch produces.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| E1 | `find_entry` (via `dataentry` mode 1) | `target_id = 100 + param2` matches no `id` in `entries[0..count)` because `param2 < 0` | `find_entry` → `NULL` ⇒ `dataentry` returns `-2` | `err_e1_e2_mode1_param2_negative` |
| E2 | `find_entry` (via `dataentry` mode 1) | `target_id` matches nothing because `param2 >= count` (one step past the last valid index) | `find_entry` → `NULL` ⇒ `dataentry` returns `-2` | `err_e1_e2_mode1_param2_ge_count` |
| E3 | `find_entry` (via `dataentry` mode 1) | `count` reached via the `param1 <= 0` default (`count = 5`) and `param2 >= 5` / `param2 < 0` | `NULL` ⇒ `-2` | `err_e3_mode1_default_count_out_of_range` |
| E4 | `find_entry` (via `dataentry` mode 1) | `100 + param2` **overflows** `int` (`param2 = INT_MAX`, `INT_MAX-99`, …) → wrapped negative target | `NULL` ⇒ `-2` | `err_e4_mode1_target_id_overflow` |
| E5 | `dataentry` mode 1, line 153 | `found->id == 0` — dead branch: ids are `100 + i` with `count` small, so never 0; documented as unreachable, asserted by never observing `-2` when the id *is* found | never taken ⇒ `result = found->value` | `err_e5_mode1_found_id_never_zero` |
| E6 | `create_entries` line 89 (`entries == NULL`) via `dataentry` mode 1, line 146 | `malloc(count * 40)` fails because `count` is huge (`param1 = INT_MAX`, `1 << 30`, …) | `create_entries` → `NULL` ⇒ `dataentry` returns `-1` | `err_e6_mode1_alloc_failure` |
| E7 | `create_entries` line 89 (`entries == NULL`) via `dataentry` mode 2, line 168 | same, through mode 2 | `NULL` ⇒ `dataentry` returns `-1` | `err_e7_mode2_alloc_failure` |
| E8 | `create_entries` line 89 (`count <= 0`) | `count <= 0`. **Unreachable from `dataentry`**: both call sites compute `count = param1 > 0 ? param1 : <5|3>`, so `count >= 1` always. Verified by never observing the leak/NULL path for any `param1 <= 0`. | not taken; `count` becomes `5` (mode 1) / `3` (mode 2) | `err_e8_count_le_zero_unreachable` |
| E9 | `dataentry` mode 1, line 146 (`count == 0`) | `count == 0`. Unreachable for the same reason as E8. | not taken | `err_e8_count_le_zero_unreachable` |
| E10 | `modify_entries` line 111 | `entries == NULL`. Unreachable from `dataentry` mode 2: line 168 already returned `-1` for NULL. Verified: mode 2 never returns the bare `-1`-from-`modify_entries` with a successful allocation. | not taken | `err_e10_modify_entries_null_unreachable` |
| E11 | `dataentry` mode 2, line 173 | `modify_entries` returns falsy (`0`) — e.g. `param2 (multiplier) == 0`, so every `value * 0 == 0` and `total == 0` | `result` stays `0`; **`param3` is NOT added** | `err_e11_mode2_multiplier_zero` |
| E12 | `dataentry` mode 2, line 173 | `modify_entries` returns `0` through **signed wraparound** of `total` even with a non-zero multiplier | `result` stays `0`, `param3` not added | `err_e12_mode2_total_wraps_to_zero` |
| E13 | `dataentry` mode 3, line 181 | `param1 < 0` (row index below range) | range check fails, `result` keeps its initial `0` | `err_e13_e16_mode3_out_of_range` |
| E14 | `dataentry` mode 3, line 181 | `param1 >= 4` (one step past the last valid row, `MAX` of `lookup_table`'s first dim) | `0` | `err_e13_e16_mode3_out_of_range` |
| E15 | `dataentry` mode 3, line 181 | `param2 < 0` (column index below range) | `0` | `err_e13_e16_mode3_out_of_range` |
| E16 | `dataentry` mode 3, line 181 | `param2 >= 3` (one step past the last valid column) | `0` | `err_e13_e16_mode3_out_of_range` |
| E17 | `dataentry` mode 3, line 182 / `calculate_lookup` line 79 | `lookup_table[row][col] == 0` → `calculate_lookup` returns `0`, `lookup_result` stays uninitialised and `param3` is not added. **Unreachable**: no cell of `lookup_table` is 0. Verified: every in-range `(param1,param2)` yields `2*cell + param3`, never `0`-from-this-branch. | not taken | `err_e17_lookup_zero_unreachable` |
| E18 | `process_name` line 61 (`dest == NULL`) | `dest == NULL`. Unreachable: the single call site passes the address of the on-stack `buffer`. | not taken | `err_e18_e19_process_name_guard_unreachable` |
| E19 | `process_name` line 61 (`*dest == '\0'`) | `*dest == '\0'`. Unreachable: `strcpy(buffer, "Default")` at line 189 runs first, so `*dest == 'D'`. | not taken; `process_name` returns `strlen("TestName") == 8` | `err_e18_e19_process_name_guard_unreachable` |
| E20 | `dataentry` default arm, line 192 | `strlen(buffer) == 0` → `result` keeps `process_name`'s return value. Unreachable: `buffer` is `"TestName"` (len 8). | not taken; `result = 8 * param1` | `err_e20_default_strlen_never_zero` |
| E21 | `dataentry` `switch`, line 188 | `mode` is not 1, 2 or 3 — i.e. **any out-of-range "enum" value** crossing the FFI boundary: `0`, `-1`, `4`, `INT_MIN`, `INT_MAX`, random | `default:` arm ⇒ `8 * param1` (wrapping) | `err_e21_mode_out_of_range_enum` |
| E22 | `dataentry` default arm, line 193 | `8 * param1` overflows `int` (`param1 = INT_MAX`, `INT_MIN`, `0x1000_0000`, …) | wrapped product as emitted by the reference compiler | `err_e22_default_overflow` |
| E23 | boundary: zero-ish lengths | `param1 == 0` for every mode (`count` defaults; `8 * 0 == 0`) | mode-dependent, see rows above | `err_e23_zero_params_all_modes` |
| E24 | boundary: extreme values | `INT_MIN` / `INT_MAX` for each of `mode`, `param1`, `param2`, `param3` independently | must match C exactly | `err_e24_extreme_values_matrix` |

Notes on constants:

* `NAME_LENGTH == 32` is the width of `DataEntry::name` and of `dataentry`'s
  local `buffer`. It is passed to `process_name` as `max_len` and then **never
  used** — `process_name` does an unbounded `strcpy`. The longest string the
  code ever writes is `"Entry_-2147483648"` (17 chars + NUL = 18 ≤ 32), so no
  overflow is reachable. No row is needed beyond E19/E20.
* `MAX_ENTRIES == 10` is never read by any check — `count` is **not** clamped to
  it. This is deliberately *not* an error row; the tests in `CONFIGS.md`
  (rows 3, 4) confirm `count > MAX_ENTRIES` is accepted.
