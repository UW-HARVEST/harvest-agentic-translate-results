# Error Surface

Mechanically derived by scanning all C source and public headers for error
returns, assertions, null/range checks, enums, and min/max constants. The only
rejection in the C implementation is the explicit null-pointer check below.
There are no error codes, error enums, assertions, length arguments, range
checks, or min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `printLine` | `line == NULL` | Return `void` without calling `puts`; produce no output | [x] |
