# Error Surface

The C API has no error enums, assertions, null checks, range checks, or
documented min/max constants. It performs one explicit rejection in `match`.
The remaining rows are the mandatory generic FFI boundaries, with outcomes
measured in isolated processes against the default C shared object.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] 1 | `match` | `total(test, bins) < threshold * total(reference, bins)` | returns `0` immediately |
| [x] 2 | `spectral_contrast` | `length == 0`, including `a == NULL` and `b == NULL` | returns `+0.0` (`0x0000000000000000`) |
| [x] 3 | `spectral_contrast` | `length == -1`, including `a == NULL` and `b == NULL` | returns `+0.0` (`0x0000000000000000`) |
| [x] 4 | `spectral_contrast` | `a == NULL`, `b` valid, and `length == 1` | process terminates with `SIGSEGV` |
| [x] 5 | `spectral_contrast` | `a` valid, `b == NULL`, and `length == 1` | process terminates with `SIGSEGV` |
| [x] 6 | `match` | `test == NULL`, `reference` valid, and `bins == 1` | process terminates with `SIGSEGV` |
| [x] 7 | `match` | `test` valid, `reference == NULL`, and `bins == 1` | process terminates with `SIGSEGV` |
| [x] 8 | `match` | valid pointers and `bins == 0` | process terminates with `SIGSEGV` |
| [x] 9 | `match` | valid pointers and `bins == -1` | process terminates with `SIGSEGV` |
| [x] 10 | `match` | non-null one-element pointers and oversized `bins == INT_MAX` | process terminates with `SIGSEGV` |

There are no public enum parameters, so no out-of-range enum value exists to
exercise.
