# Error Surface

The complete C source was searched for error-return statements and macros,
assertions, null/range checks, error enums, and min/max constants. It contains
none. Neither exported function reports or rejects invalid input.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|

Generic FFI boundary behavior that is not an explicit C rejection is tracked
separately in the differential tests. In particular, a null `in` pointer has
undefined C behavior and faults on this build; both shared libraries are tested
in subprocesses for matching behavior. There are no length or enum parameters.
