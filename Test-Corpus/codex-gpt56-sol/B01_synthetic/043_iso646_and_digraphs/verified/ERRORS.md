# Error Surface

The complete C source was searched for `return`, `assert`, `NULL`, enums,
conditionals, switches, range checks, and error macros. It contains no input
rejection or error branch. The sole return is the unconditional `return 0`
from `main`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|

Generic pointer, length, and enum boundary cases are not applicable: neither
public entry point accepts a pointer, length, or enum. `driver` accepts two
full-range C `int` values. The observable failed-input states of the two
unchecked `scanf` calls are valid execution paths and are listed in
`CONFIGS.md`.

Phase C status: complete (zero source-derived rejection rows). The differential
test additionally covers zero, `INT_MIN`, `INT_MAX`, immediate EOF,
nonmatching input, and a missing/nonmatching second value.
