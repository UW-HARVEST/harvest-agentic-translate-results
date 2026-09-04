# Error-Surface Table

The complete C source was mechanically searched for error returns, all return
statements, assertions, null checks, explicit range checks, error identifiers,
and min/max constants:

```text
rg -n -i 'RETURN_ERROR|return\s+(-1|NULL)|\breturn\b|\bassert\s*\(|error|invalid|range|minimum|maximum|\bmin\b|\bmax\b|==\s*NULL|!=\s*NULL|!\s*[A-Za-z_][A-Za-z0-9_]*|<|>' \
  ../c_src/include ../c_src/src
```

No rejection or error path exists. The sole public function accepts one
by-value `float`, returns `void`, and has no pointer, length, enum, range, or
state argument. Therefore the generic null-pointer, zero/oversized-length, and
out-of-range-enum cases are not applicable.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | — | No invalid-input condition exists in the C API. | — |

Phase C status: [x] complete (zero applicable rejection rows).
