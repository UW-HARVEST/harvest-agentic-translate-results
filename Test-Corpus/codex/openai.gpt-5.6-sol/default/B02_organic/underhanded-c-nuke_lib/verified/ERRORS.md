# Error surface

Mechanical scan used:

```text
rg -n 'return|assert|NULL|if\s*\(|switch\s*\(|#if|enum' ../c_src/include ../c_src/src
```

The C source has no `assert`, null check, enum, explicit length range, or
documented maximum. Row 1 is its only explicit rejection branch. Rows 2–11
track the mandatory generic FFI boundaries; signal results were measured in
isolated processes so undefined memory accesses cannot kill the test runner.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] 1 | `match` | `total(test, bins) < threshold * total(reference, bins)` | returns `0` immediately, without preprocessing |
| [x] 2 | `spectral_contrast` | `a == NULL`, `b == NULL`, `length == 0` | returns positive `0.0` (`0x0000000000000000`) |
| [x] 3 | `spectral_contrast` | `a == NULL`, `b == NULL`, `length < 0` | returns positive `0.0` (`0x0000000000000000`) |
| [x] 4 | `spectral_contrast` | `a == NULL`, non-null `b`, `length == 1` | process terminates with `SIGSEGV` |
| [x] 5 | `spectral_contrast` | non-null `a`, `b == NULL`, `length == 1` | process terminates with `SIGSEGV` |
| [x] 6 | `spectral_contrast` | one-element non-null buffers, `length == INT_MAX` | process terminates with `SIGSEGV` while reading past the buffers |
| [x] 7 | `match` | `test == NULL`, non-null `reference`, `bins == 1` | process terminates with `SIGSEGV` |
| [x] 8 | `match` | non-null `test`, `reference == NULL`, `bins == 1` | process terminates with `SIGSEGV` |
| [x] 9 | `match` | `test == NULL`, `reference == NULL`, `bins == 0` | process terminates with `SIGSEGV` after the zero-length VLA path writes `v[-1]` |
| [x] 10 | `match` | non-null one-element buffers, `bins == -1` | process terminates with `SIGSEGV` after the negative-length VLA path writes `v[-1]` |
| [x] 11 | `match` | non-null one-element buffers, `bins == INT_MAX` | process terminates with `SIGSEGV` from the oversized VLA/access |

There are no enum parameters, so no out-of-range enum case exists. There is
also no documented finite valid range for `bins`, `length`, or `threshold`;
zero, negative, and `INT_MAX` cover the applicable one-step/boundary classes.
