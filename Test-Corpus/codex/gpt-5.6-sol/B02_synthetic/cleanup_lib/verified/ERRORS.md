# Error Surface

Mechanically derived from every `if`, null check, and rejection branch in
`c_src/src/lib.c`. The public API has no lengths, enums, assertions,
error-return macros, range checks, or min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `cleanup` | `strncmp(input_str, expected_str, strlen(expected_str)) != 0`; unreachable with the compiled-in identical `"VALID"` literals under the normal C runtime, forced in tests by interposing `strncmp` | prints `Input string validation failed.\n`, skips number processing/allocation, calls `cleanup_resources(NULL)`, returns `0` | [x] |
| 2 | `cleanup` | `malloc(50 * sizeof(char)) == NULL`, forced in tests by failing the next 50-byte allocation | prints `Memory allocation failed.\n`, calls `cleanup_resources(NULL)`, returns the already-computed result | [x] |
| 3 | `cleanup_resources` | `dynamic_str == NULL` | performs no free and returns normally | [x] |

Generic FFI boundaries not represented by additional C rejection branches:

- `print_result(NULL, result)` is accepted by this build's glibc `printf` and
  prints `(null): result`; it is covered differentially.
- There are no pointer-plus-length, enum, or integer range contracts in this
  API, so zero/oversized lengths and out-of-range enum values do not apply.
