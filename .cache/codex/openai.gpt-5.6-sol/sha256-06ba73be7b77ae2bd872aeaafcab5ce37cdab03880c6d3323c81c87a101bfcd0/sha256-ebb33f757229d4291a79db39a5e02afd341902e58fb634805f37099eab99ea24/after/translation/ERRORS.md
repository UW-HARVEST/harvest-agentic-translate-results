# Error Surface

Derived from all explicit rejection branches in `../c_src/include/` and
`../c_src/src/`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `smallestValue` | `head == NULL` | returns `-1` | [x] |

There are no length parameters, enums, range checks, assertions, error enums,
or other explicit rejection paths. Invalid non-null pointers, cyclic lists,
and dangling `next` pointers do not have defined rejection behavior in C and
are therefore not error-surface inputs.
