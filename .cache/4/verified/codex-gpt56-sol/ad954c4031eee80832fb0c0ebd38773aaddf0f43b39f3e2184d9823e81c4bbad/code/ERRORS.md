# Error Surface

Mechanically derived from every conditional return, error macro, assertion,
null check, range check, and min/max constant in `c_src/src` and
`c_src/include`.

| # | function | trigger (the exact invalid input/condition) | expected C result | Status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `smallestValue` | `head == NULL` | returns `-1` | [x] |

There are no length parameters, enums, assertions, error macros, explicit range
checks, or min/max constants in the public C API. Every `int` value is valid.
Non-null invalid pointers and cyclic lists violate the C function's memory/list
preconditions and do not have defined rejection behavior.
