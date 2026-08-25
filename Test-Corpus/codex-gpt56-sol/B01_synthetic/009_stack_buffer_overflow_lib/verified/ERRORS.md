# Error Surface

Mechanically derived from the null/range conditions in
`c_src/src/driver.c`. The API returns `void`, so rejection is observable
through exact stdout bytes.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `printLine` | `line == NULL` (line 31) | Return without output. | [x] |
| 2 | `bad`; `driver` via `badData` | `data < 0`, the false side of `data >= 0` (lines 46, 55-58) | Output `ERROR: Array index is negative.\n`. | [x] |
| 3 | `good`; `driver` via `goodData` | `data < 0`, the lower-bound failure of `data >= 0 && data < 10` (lines 85, 94-97) | After `goodG2B` output, output `ERROR: Array index is out-of-bounds\n`. | [x] |
| 4 | `good`; `driver` via `goodData` | `data >= 10`, the upper-bound failure of `data >= 0 && data < 10` (lines 85, 94-97) | After `goodG2B` output, output `ERROR: Array index is out-of-bounds\n`. | [x] |
| 5 | `bad`; `driver` via `badData` | `data == 10`, one past the `int buffer[10]` valid index range; C has no upper-bound rejection (lines 45-53) | C performs its unchecked write and then attempts to output ten array elements; compare the actual process result and bytes exactly. | [x] |

The similar condition in `goodG2B` has a fixed local `data = 7`; its error
side is unreachable from every public input. It is covered on its reachable
side by calls to `good` and `driver`. There are no assertions, return-error
macros, error enums, pointer/length pairs, or public enum parameters.
