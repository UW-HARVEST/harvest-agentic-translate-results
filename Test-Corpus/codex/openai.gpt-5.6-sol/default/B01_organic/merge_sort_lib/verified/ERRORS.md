# Error Surface

Mechanical source scan covered `../c_src/include/lib.h` and
`../c_src/src/lib.c` for error-return statements and macros, `assert`, null
checks, range checks, error enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | Tested |
|---|----------|----------------------------------------------|-------------------|--------|

There are zero explicit rejection paths. `merge_sort` returns `void` and does
not validate either pointer or `size`. In particular:

- A negative `size` is converted to `size_t` by the `memcpy` byte-count
  expression; it is not rejected.
- Null, overlapping, undersized, or otherwise invalid buffers are not checked.
- There are no enum arguments, assertions, documented ranges, error codes, or
  sentinel returns.

Invalid pointer/length combinations invoke C undefined behavior rather than a
library-defined rejection and therefore have no expected C result to compare.
The defined zero-length and boundary-size cases are tracked in `CONFIGS.md`.

## Generic Boundary Probes

These mandatory probes are kept separate from the zero-row source-derived
error table because the invalid cases have undefined behavior, not C-defined
error results. Faulting calls run in isolated subprocesses.

| Boundary | Differential result | Tested |
|----------|---------------------|--------|
| Both pointers null with `size == 0` | Both calls return | [x] |
| Input pointer null with `size == 0` | Both calls return | [x] |
| Scratch pointer null with `size == 0` | Both calls return | [x] |
| Input pointer null with `size == 1` | Same abnormal process termination | [x] |
| Scratch pointer null with `size == 1` | Same abnormal process termination | [x] |
| One-element buffers with `size == INT_MAX` | Same abnormal process termination | [x] |
| One-element buffers with `size == -1` | Same abnormal process termination | [x] |

There are no enum parameters or documented value ranges, so out-of-range enum
and one-step-past-documented-range probes do not apply.
