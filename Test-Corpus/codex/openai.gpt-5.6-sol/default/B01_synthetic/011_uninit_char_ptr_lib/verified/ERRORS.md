# Error Surface

Mechanical source scan:

```text
rg -n -i 'RETURN_ERROR|return[[:space:]]+(-1|NULL)|assert[[:space:]]*\(|if[[:space:]]*\(|switch[[:space:]]*\(|case[[:space:]]+|typedef[[:space:]]+enum|enum|min|max|NULL' c_src/include c_src/src -g '*.[ch]'
```

The scan finds one explicit null check and no error-return statements, asserts,
enums, range checks, or min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `printLine` | `line == NULL` | Return `void` without writing any output | [x] |

Generic FFI boundaries not represented by C parameters:

- There are no length parameters, so zero or oversized lengths do not apply.
- There are no C enum parameters, so out-of-range enum values do not apply.
- `driver` accepts the full range of C `int`; zero and nonzero are both valid.
- `bad` and `good` take no pointers or scalar inputs.
