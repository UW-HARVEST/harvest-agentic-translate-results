# Error Surface

The C source was mechanically searched for `RETURN_ERROR`, `return -1`,
`return NULL`, error enums, `assert`, `if` checks, null checks, and range
checks. None occur. `call_predict` accepts one C `int`, so pointer, length, and
enum boundary categories do not apply.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|

There are no C rejection paths. Values outside the named switch cases are
accepted by the default branch and are covered as a valid-path configuration
in `CONFIGS.md`.

Completion:

- [x] Every C rejection path has a passing differential test (empty set).
- [x] Generic pointer/length/enum error boundaries are not applicable to this ABI.
