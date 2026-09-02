# CONFIGS.md — configuration-surface table

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

| axis | where | distinct values the C distinguishes |
|------|-------|-------------------------------------|
| `mode` | `switch (mode)` in `dataentry` | `1`, `2`, `3`, everything else (`default`) |
| `param1` sign | `count = param1 > 0 ? param1 : 5` (case 1) / `: 3` (case 2) | `> 0` (explicit count) vs `<= 0` (default count 5 or 3) |
| `count` shape | `create_entries` loop, `find_entry` / `modify_entries` pointer walk | `1` (one), `2`/`3`/`5` (few), `10` (`MAX_ENTRIES`), `>10` (many), huge (malloc failure) |
| `param2` as index (case 1) | `find_entry(entries, count, 100 + param2)` | `0` (first), middle, `count-1` (last), `count` (one past), `> count`, `< 0` |
| `param2` as multiplier (case 2) | `modify_entries(entries, count, param2)` | `0`, `1`, `-1`, small +, small -, `INT_MAX`, `INT_MIN` (wraparound) |
| `param1` as row (case 3) | `param1 >= 0 && param1 < 4` then `lookup_table[row][...]` | `0`, `1`, `2`, `3` (all valid rows) |
| `param2` as col (case 3) | `param2 >= 0 && param2 < 3` then `lookup_table[...][col]` | `0`, `1`, `2` (all valid cols) |
| `param3` additive | `result += param3` (case 2), `result = lookup_result + param3` (case 3) | `0`, positive, negative, `INT_MAX`, `INT_MIN` (overflow) |
| `param1` as multiplier (default) | `result = count * param1` where `count == 8` | `0`, positive, negative, `INT_MAX`, `INT_MIN` (overflow) |
| name formatting | `sprintf(temp_name, "Entry_%d", base_id + i)` | 3-digit ids (`100..`, `200..`), 4-digit, 5+-digit — different `strlen` inside `NAME_LENGTH` |
| `strcpy` into `buffer[32]` | case 1 `strcpy(buffer, found->name)`; default `strcpy(buffer, "Default")` | short vs longest reachable name |
| `#ifdef` / build flags | none — `grep -c '#if' c_src/src/lib.c` → 0 | single configuration |
| cargo features | `translation/Cargo.toml` declares no `[features]` | one combination: default (= no features) |

There are no runtime option setters, no init/teardown functions, and no
`#ifdef` branches: `dataentry(mode, param1, param2, param3)` is the entire
public API, so the "options" are the four integer parameters and the
configuration surface is their pruned cross-product. Both the composed
top-level operation and the low-level helpers it drives (`create_entries` →
`find_entry` / `modify_entries` → `calculate_lookup` → `process_name`) are
exercised through it, since every helper is `static` and reachable only via
`dataentry`.

## Rows

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|-------------------------------------------|-----|
| 1  | `dataentry` → `create_entries`, `find_entry` | mode 1, `param1 <= 0` (default count 5), `param2` in `[0,5)` — hit, all indices | [x] |
| 2  | `dataentry` → `create_entries`, `find_entry` | mode 1, `param1 == 1` (count 1, single element), `param2 == 0` — first == last | [x] |
| 3  | `dataentry` → `create_entries`, `find_entry` | mode 1, `param1 == 2` (count 2), `param2` = 0 and 1 — boundary pair | [x] |
| 4  | `dataentry` → `create_entries`, `find_entry` | mode 1, `param1 == 10` (`MAX_ENTRIES`), `param2` over all `[0,10)` | [x] |
| 5  | `dataentry` → `create_entries`, `find_entry` | mode 1, `param1` many (`100`, `1000`, `10000`), randomized in-range `param2` — 3/4/5-digit `Entry_%d` names | [x] |
| 6  | `dataentry` → `create_entries`, `find_entry` | mode 1, `param1 > 0` randomized, `param2` randomized (in- and out-of-range mixed) | [x] |
| 7  | `dataentry` → `create_entries`, `find_entry` | mode 1, `param3` randomized (must be ignored entirely on this path) | [x] |
| 8  | `dataentry` → `create_entries`, `modify_entries` | mode 2, `param1 <= 0` (default count 3), `param2 == 1` (identity multiplier), `param3 == 0` | [x] |
| 9  | `dataentry` → `create_entries`, `modify_entries` | mode 2, count 1 / 2 / 3 / 10, `param2` small positive, `param3 == 0` | [x] |
| 10 | `dataentry` → `create_entries`, `modify_entries` | mode 2, `param2 == 0` → every product 0 → total 0 → `param3` NOT added | [x] |
| 11 | `dataentry` → `create_entries`, `modify_entries` | mode 2, `param2` negative (incl. `-1`), `param3` randomized | [x] |
| 12 | `dataentry` → `create_entries`, `modify_entries` | mode 2, `param2` large so `total` overflows `int` (signed wraparound accumulation) | [x] |
| 13 | `dataentry` → `create_entries`, `modify_entries` | mode 2, count many (`100`, `1000`, `10000`) — long accumulation, multi-digit ids | [x] |
| 14 | `dataentry` → `create_entries`, `modify_entries` | mode 2, `param3` at `INT_MAX` / `INT_MIN` — overflow in `result += param3` | [x] |
| 15 | `dataentry` → `create_entries`, `modify_entries` | mode 2, fully randomized `param1`/`param2`/`param3` | [x] |
| 16 | `dataentry` → `calculate_lookup` | mode 3, full 4x3 cross-product of valid `(param1, param2)`, `param3 == 0` | [x] |
| 17 | `dataentry` → `calculate_lookup` | mode 3, full 4x3 cross-product with `param3` randomized (incl. negative) | [x] |
| 18 | `dataentry` → `calculate_lookup` | mode 3, valid indices with `param3` at `INT_MAX` / `INT_MIN` — `lookup_result + param3` overflow | [x] |
| 19 | `dataentry` → `process_name` | mode 0 (default), `param1` randomized — `8 * param1` | [x] |
| 20 | `dataentry` → `process_name` | mode negative / `> 3` / `INT_MIN` / `INT_MAX` (default arm via out-of-range mode), `param1` randomized | [x] |
| 21 | `dataentry` → `process_name` | default arm, `param1 == 0` (product 0) and `param1` at `INT_MAX` / `INT_MIN` (overflow in `8 * param1`) | [x] |
| 22 | `dataentry` (all arms) | full randomized 4-tuple sweep across all modes, fixed seed, 200k cases | [x] |
| 23 | `dataentry` (all arms) | exhaustive small-domain sweep: `mode` and all params over `[-6, 12]` | [x] |
| 24 | `dataentry` (all arms) | every parameter at `INT_MIN` / `INT_MIN+1` / `-1` / `0` / `1` / `INT_MAX-1` / `INT_MAX` cross-product | [x] |
| 25 | `dataentry` → `create_entries` | mode 1 and mode 2 with `param1` huge (`INT_MAX`, `INT_MAX/2`, `0x4000_0000`) — allocation-failure path | [x] |
| 26 | `dataentry` → `create_entries` | mode 1 and mode 2, power-of-two `param1` ladder crossing the allocator's success/failure boundary — the flip point must be identical, not merely "both fail somewhere" | [x] |

All 26 rows verified under the only cargo feature combination (default / none).
