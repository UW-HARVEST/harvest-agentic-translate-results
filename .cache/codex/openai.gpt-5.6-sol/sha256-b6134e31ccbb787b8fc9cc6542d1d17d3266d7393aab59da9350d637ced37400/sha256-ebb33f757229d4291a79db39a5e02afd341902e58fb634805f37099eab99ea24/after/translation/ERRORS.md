# Error-surface table

Mechanical source scan:

```text
rg -n 'RETURN_ERROR|return[[:space:]]+(-1|NULL)|assert[[:space:]]*\(|\
if[[:space:]]*\(|switch[[:space:]]*\(|NULL|MIN|MAX|enum' \
../c_src/include ../c_src/src
```

The C source contains no explicit rejection branches, error-return macros,
assertions, null checks, range checks, length parameters, enums, or min/max
constants. The rows below are the mandatory generic FFI pointer boundaries.
Because C does not reject them with an error value, equivalence is measured by
the process termination status from an isolated child.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `foo` | `in == NULL`, with any non-NUL search byte | no sentinel; child terminates from invalid memory access | [x] |
| 2 | `driver` | `in == NULL` | no sentinel; child terminates from invalid memory access before producing a complete result | [x] |
| 3 | `foo` | `c == '\0'` and the string terminator is at the end of a readable page | the first `strchr` finds the terminator, then the loop advances into the guard page and the child terminates from invalid memory access | [x] |

Generic boundary applicability:

- Null pointers: rows 1-2.
- Zero lengths: not applicable; no public entry point accepts a length.
- Oversized lengths: not applicable; no public entry point accepts a length.
- One-past-range values: not applicable; no bounded numeric parameter exists.
- Out-of-range enums: not applicable; no public entry point accepts an enum.
