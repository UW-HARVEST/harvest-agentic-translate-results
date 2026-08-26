# Error Surface

The table is derived from the guards, allocation check, and fallback branches
in `c_src/src/lib.c`. The C source has no assertions, enum types, named
min/max constants, or explicit pointer-validation branches. A null pointer is
listed only where a guard returns before dereferencing it; null pointers on
dereferencing paths have undefined C behavior and no result to compare. Those
paths are nevertheless exercised in isolated subprocesses, where Rust and C
are required to terminate with the same signal.

The `arity` binary follows its definition's `unsigned char len`, despite the
public header declaring `int len`. Thus "effective length" below means the low
eight bits of the caller's `int`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `shift_array` | `positions <= 0`, so `positions > 0 && positions < size` is false | Return without changing the array | [x] |
| 2 | `shift_array` | `positions > 0 && positions >= size` | Return without changing the array | [x] |
| 3 | `apply_bitmask` | `operation < 0` (below the handled range `0..=3`) | Return `value` unchanged | [x] |
| 4 | `apply_bitmask` | `operation > 3` (above the handled range `0..=3`) | Return `value` unchanged | [x] |
| 5 | `compare_allocations` | Either `malloc(sizeof(int))` returns `NULL` | Free both allocation results and return `-1` | [x] |
| 6 | `arity` | Effective length is `0` | Return `-1` without reading `params` | [x] |
| 7 | `arity` | Effective length is `1` | Return `-1` without reading `params` | [x] |
| 8 | `arity` | `params == NULL` and effective length is less than `2` | Return `-1` without reading `params` | [x] |
