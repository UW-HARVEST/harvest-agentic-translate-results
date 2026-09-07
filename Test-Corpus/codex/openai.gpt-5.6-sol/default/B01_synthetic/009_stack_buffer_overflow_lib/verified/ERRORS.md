# Error Surface

This table is derived from every C null/range check and every explicit error
string in `src/driver.c`. There are no return-error statements, error enums,
assertions, or error return values; every public function returns `void`, so
the observable result is its exact stdout bytes.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `printLine` | `line == NULL` | Return without writing any bytes. | [x] |
| 2 | `bad` | `data < 0` | Write `ERROR: Array index is negative.\n`. | [x] |
| 3 | `good` | `data < 0` (the `goodB2G` lower-bound failure) | First write the fixed ten-line `goodG2B` buffer, then `ERROR: Array index is out-of-bounds\n`. | [x] |
| 4 | `good` | `data >= 10` (the `goodB2G` upper-bound failure) | First write the fixed ten-line `goodG2B` buffer, then `ERROR: Array index is out-of-bounds\n`. | [x] |
| 5 | `driver` | `goodData < 0` | Emit the driver framing, the fixed `goodG2B` buffer, the `goodB2G` out-of-bounds error, and then the selected `bad` result. | [x] |
| 6 | `driver` | `goodData >= 10` | Emit the driver framing, the fixed `goodG2B` buffer, the `goodB2G` out-of-bounds error, and then the selected `bad` result. | [x] |
| 7 | `driver` | `badData < 0` | Emit the driver framing and selected `good` result, then `ERROR: Array index is negative.\n`, then `Finished bad()\n`. | [x] |

`goodG2B` contains an error branch for `data < 0`, but `data` is a fixed local
constant equal to 7. No caller-controlled input can trigger that branch.

`bad` intentionally has no upper-bound rejection. `bad(10)` and the equivalent
`driver` composition are covered separately as generic one-past-boundary
differential tests, but they are not error-surface rows because C takes the
nonnegative branch.

Generic FFI boundaries:

- The only pointer-taking API is `printLine`; its null boundary is row 1.
- There are no length parameters or enums.
- Integer zero is valid and is covered by the valid-path matrix.
- `good(10)` is the one-past-upper-bound case and is row 4.
- `bad(10)` and `driver(_, 10)` exercise the missing-upper-check boundary.
