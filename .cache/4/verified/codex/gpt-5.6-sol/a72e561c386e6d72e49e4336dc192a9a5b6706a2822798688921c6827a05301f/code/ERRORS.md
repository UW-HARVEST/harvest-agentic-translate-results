# Error surface

Mechanical searches covered `return -1`, `return NULL`, error macros and enums,
assertions, preprocessor conditionals, explicit range/null checks, and min/max
constants in every C source and public header.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rows: `merge_sort` returns `void` and the C implementation has no
error return, assertion, range check, null check, enum input, or documented
minimum/maximum. Invalid pointers, overlapping buffers, negative sizes, and
sizes larger than the supplied allocations reach `memcpy` or pointer indexing
and have undefined C behavior; they are not C rejection paths with a result
that can be compared.

## Generic FFI boundaries

| Boundary | Coverage |
|----------|----------|
| zero length with valid non-null buffers | valid-path configuration row 1 |
| null pointers | isolated exact process-outcome differential probe |
| oversized length | valid allocation in configuration row 12; invalid allocation in isolated exact process-outcome probe |
| one past a documented range | not applicable; no documented range |
| out-of-range enum | not applicable; the API has no enum parameter |

## Completion

- [x] Every source-defined rejection row has a differential test (zero rows).
- [x] Generic applicable FFI boundaries have matching differential outcomes.
