# Error Surface

Mechanically derived from every rejecting `return` and its guarding condition
in `c_src/src/slicing.c`. The comparisons against `size_t len` apply C's usual
integer conversions, so a negative `int` index is converted to a large
unsigned value.

| # | function | trigger (the exact invalid input/condition) | expected C result | Test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `slice` | `start_ptr != NULL` and `(size_t)*start_ptr > strlen(mystr)`; this includes every negative `*start_ptr` | Writes `Error: start is off the end of the string!\n`; returns `1` | [x] |
| 2 | `slice` | Start passed row 1, `stop_ptr != NULL`, and `(size_t)*stop_ptr > strlen(mystr)`; this includes every negative `*stop_ptr` | Writes `Error: stop is off the end of the string!\n`; returns `1` | [x] |
| 3 | `slice` | Start and stop passed rows 1-2, `stop_ptr != NULL`, and `*stop_ptr <= start`, where `start` is `*start_ptr` or `0` when `start_ptr == NULL` | Writes `Error: stop must come after start!\n`; returns `1` | [x] |

There are no assertions, enums, min/max constants, explicit `mystr == NULL`
checks, or length parameters in the C API. Passing `mystr == NULL` reaches
`strlen(NULL)` and has undefined behavior; it is covered separately by a
process-isolated parity test rather than represented as an explicit C
rejection.
