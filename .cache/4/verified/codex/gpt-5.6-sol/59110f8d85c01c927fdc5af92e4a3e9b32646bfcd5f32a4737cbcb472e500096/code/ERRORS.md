# Error Surface

The complete C source and public header were mechanically scanned for error
returns, null checks, assertions, range checks, enums, and min/max constants:

```text
RETURN_ERROR
return -1
return NULL
assert(...)
if (...) / switch (...)
enum
NULL
MIN / MAX
```

No rejection or error path exists. The API has no pointer, length, enum, or
fallible return-value input, so the error-surface table has zero rows.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

Generic boundary review: `long_exec` takes one `unsigned int`, for which every
bit pattern is valid. `perform_expensive_operations` takes no arguments.

Phase C status: [x] complete (zero error-surface rows).
