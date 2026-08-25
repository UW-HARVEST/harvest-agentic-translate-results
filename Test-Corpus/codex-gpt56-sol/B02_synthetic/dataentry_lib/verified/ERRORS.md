# Error Surface

This table comes from every null/range/error-return branch in
`c_src/src/lib.c`. `dataentry` is the only public entry point. Rows marked
private-invariant describe branches in `static` helpers which no public input
can reach because every call site establishes the opposite condition.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `find_entry` | No entry in `[entries, entries + count)` has `id == target_id` (line 55). Public trigger: mode 1 with `param2 < 0` or `param2 >= count`. | `NULL`; `dataentry` maps it to `-2`. | [x] differential |
| 2 | `process_name` | `dest == NULL || *dest == '\0'` (lines 61-62). Private invariant: the only call passes non-null `buffer` containing `"Default"`. | `-1` | [x] call-site invariant; no public trigger |
| 3 | `create_entries` | `malloc(count * sizeof(DataEntry)) == NULL || count <= 0` (lines 87-90). Public callers replace nonpositive counts with 5 or 3; allocation failure remains externally observable. | `NULL`; mode 1 or 2 maps it to `-1`. | [x] differential allocation failure and default-count invariant |
| 4 | `modify_entries` | `entries == NULL` (lines 111-112). Private invariant: mode 2 returns before calling after a failed allocation. | `-1` | [x] call-site invariant; no public trigger |
| 5 | `dataentry` mode 1 | `entries == NULL || count == 0` (lines 146-148). `count == 0` is prevented by the default-to-5 expression; allocation failure is reachable. | `-1` | [x] differential allocation failure and default-count invariant |
| 6 | `dataentry` mode 1 | `found == NULL || found->id == 0` (lines 153-155). A missing target is reachable; `id == 0` requires impractically traversing wrapped signed arithmetic and is not a defined-C input path. | `-2` | [x] differential missing target; `id == 0` invariant |
| 7 | `dataentry` mode 2 | `entries == NULL` (lines 168-170), caused by allocation failure. | `-1` | [x] differential allocation failure |
| 8 | `dataentry` mode 3 | `param1 < 0` fails the row lower bound (line 181). | `0` | [x] differential |
| 9 | `dataentry` mode 3 | `param1 >= 4` fails the row upper bound (line 181). | `0` | [x] differential |
| 10 | `dataentry` mode 3 | `param2 < 0` fails the column lower bound (line 181). | `0` | [x] differential |
| 11 | `dataentry` mode 3 | `param2 >= 3` fails the column upper bound (line 181). | `0` | [x] differential |

There are no pointer, length, or enum parameters in the public API.
`MAX_ENTRIES` is defined as 10 but never read, so it imposes no rejection.
`NAME_LENGTH` is 32, while `process_name` ignores its `max_len` argument; the C
code therefore has no corresponding length check to add to this table.
`calculate_lookup` can return 0 for a zero table cell, but all 12 compiled table
cells are nonzero, so no input can trigger that branch.
