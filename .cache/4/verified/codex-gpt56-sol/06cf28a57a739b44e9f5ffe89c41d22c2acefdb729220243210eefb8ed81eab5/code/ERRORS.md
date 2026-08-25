# Error surface

The following mechanical search was applied to all files under `c_src/src/`
and `c_src/include/`:

```text
RETURN_ERROR
return -1
return NULL
error-enum returns
assert(...)
if/switch/case
NULL
MIN/MAX constants
```

It found no rejection branches, error returns, assertions, range checks, null
checks, min/max constants, pointer arguments, length arguments, or enum
arguments. The only public API is the infallible `void driver(int x)`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

There are zero error-surface rows. Generic pointer, length, and out-of-range
enum cases are not applicable to this scalar-only API.
