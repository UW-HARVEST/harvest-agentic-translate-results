# Error Surface

Mechanically derived from all `if` and error-return branches in
`c_src/src/lib.c`. There are no assertions, enums, explicit range checks,
length parameters, or min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `custom_strdup` | `str == NULL` | returns `NULL` without calling `strlen` or `malloc` | [x] |
| 2 | `custom_strdup` | `malloc(strlen(str) + 1) == NULL` | returns `NULL` | [x] |

Generic FFI boundary applicability:

- Null pointers are covered by row 1.
- Zero and oversized lengths do not apply because the API has no length
  parameter.
- Out-of-range and one-past-range values do not apply because the API has no
  numeric or enum parameters.
