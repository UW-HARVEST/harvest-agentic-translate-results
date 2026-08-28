# Error Surface

The public header declares one function:
`int dataentry(int mode, int param1, int param2, int param3)`.
It has no pointer, length, or enum parameters. Consequently, generic null
pointer, oversized-length, and invalid-enum FFI cases do not exist for this
API.

## Publicly Reachable Rejections

Each row is a distinct invalid condition accepted at the exported FFI
boundary. The status box is checked only after the corresponding differential
test passes.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `dataentry`, mode 1 | effective count is 5 (`param1 <= 0`) and `param2 < 0`, so `find_entry` returns `NULL` | [x] `-2` |
| 2 | `dataentry`, mode 1 | effective count is 5 (`param1 <= 0`) and `param2 >= 5`, so `find_entry` returns `NULL` | [x] `-2` |
| 3 | `dataentry`, mode 1 | `param1 > 0` and `param2 < 0`, so the target precedes the allocated ID range | [x] `-2` |
| 4 | `dataentry`, mode 1 | `param1 > 0` and `param2 >= param1`, so the target follows the allocated ID range | [x] `-2` |
| 5 | `dataentry`, mode 3 | `param1 < 0` (row below the valid `0..=3` range) | [x] `0` |
| 6 | `dataentry`, mode 3 | `param1 >= 4` (row above the valid `0..=3` range) | [x] `0` |
| 7 | `dataentry`, mode 3 | valid row and `param2 < 0` (column below the valid `0..=2` range) | [x] `0` |
| 8 | `dataentry`, mode 3 | valid row and `param2 >= 3` (column above the valid `0..=2` range) | [x] `0` |

## Non-Public Guard Inventory

These checks were also found mechanically in `src/lib.c`. They cannot be
constructed through the exported ABI: their functions are `static`, `nm -D`
does not expose them, and `dataentry` supplies arguments that make the
conditions unreachable except for nondeterministic allocator failure.

| # | static function | exact guard | C result / reachability |
|---|-----------------|-------------|-------------------------|
| G1 | `find_entry` | no entry has `id == target_id` | `NULL`; reaches public rows 1-4 |
| G2 | `process_name` | `dest == NULL` | `-1`; wrapper always passes `buffer` |
| G3 | `process_name` | `*dest == '\0'` | `-1`; wrapper initializes `buffer` to `"Default"` |
| G4 | `calculate_lookup` | selected table value is zero | `0`; all 12 fixed table values are nonzero |
| G5 | `create_entries` | `malloc` returns `NULL` | `NULL`; only nondeterministic allocation failure |
| G6 | `create_entries` | `count <= 0` | `NULL`; wrapper substitutes 5 or 3 before the call |
| G7 | `modify_entries` | `entries == NULL` | `-1`; wrapper calls it only after successful allocation |
| G8 | `dataentry`, mode 1 | `create_entries` returns `NULL` | `-1`; only nondeterministic allocation failure |
| G9 | `dataentry`, mode 1 | `count == 0` after fallback selection | `-1`; unreachable because count is positive |
| G10 | `dataentry`, mode 1 | `found->id == 0` | `-2`; generated IDs begin at 100 for defined-size calls |
| G11 | `dataentry`, mode 2 | `create_entries` returns `NULL` | `-1`; only nondeterministic allocation failure |

`MAX_ENTRIES` is defined as 10 but is never read; it imposes no public range
check. `NAME_LENGTH` is the fixed internal array width, not an API length.
There are no assertions or error enums.

Verification command:

```sh
timeout 600 cargo test --release --test differential -- --test-threads=1
```
