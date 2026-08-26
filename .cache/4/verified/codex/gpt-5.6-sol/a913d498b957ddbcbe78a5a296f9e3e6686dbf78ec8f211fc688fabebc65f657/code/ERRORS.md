# Error Surface

Mechanical search covered `return`, `if`, `assert`, `NULL`, error macros, enum
definitions, and min/max constants in `c_src/include/driver.h` and
`c_src/src/driver.c`. The library has no error code, error sentinel, assertion,
null check, enum input, or explicit range check. Its one input-rejection branch
rejects the next token while retaining all prior successfully parsed tokens.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|
| E1 | `driver` | `sscanf(in, "%d%zn", ...) != 1`: the next input position is EOF or does not begin a conversion accepted by `%d` after optional whitespace | stop parsing; print the last accepted integer, or `0\n` when no integer was accepted | [x] |

Invalid non-null buffer extents and null pointers paired with positive lengths
are not rejected by C; they violate the functions' implicit memory preconditions
and invoke undefined behavior. Defined null/boundary cases that do not
dereference a pointer are included in `CONFIGS.md`.
