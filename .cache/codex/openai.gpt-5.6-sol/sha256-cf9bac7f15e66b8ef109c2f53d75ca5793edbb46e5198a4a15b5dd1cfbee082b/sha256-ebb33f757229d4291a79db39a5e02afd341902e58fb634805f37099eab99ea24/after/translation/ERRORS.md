# Error surface

Mechanical scan covered every `if`, `switch`, `return`, null token, assertion,
enum, and min/max token in `../c_src/include` and `../c_src/src`. There are no
error enums, assertions, explicit numeric ranges, caller-supplied lengths, or
error return codes.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `cleanup` | `strncmp(input_str, expected_str, strlen(expected_str)) != 0` | prints `Input string validation failed.\n`, calls `cleanup_resources(NULL)`, returns `0`; unreachable through the current public arguments because both strings are the same internal literal | [x] |
| 2 | `cleanup` | `malloc(50) == NULL` | prints `Memory allocation failed.\n`, calls `cleanup_resources(NULL)`, returns the accumulated switch result | [x] |
| 3 | `cleanup_resources` | `dynamic_str == NULL` | explicit null branch: performs no operation and returns `void` | [x] |
| 4 | `print_result` | generic FFI null boundary: `label == NULL` (the C source performs no rejection check) | on the test platform's glibc, `printf("%s: %d\n", NULL, result)` prints `(null): <result>\n` and returns normally | [x] |

Rows 1–3 come from explicit C branches. Row 4 is included by the mandatory
generic FFI-boundary rule. No oversized-length or out-of-range-enum cases exist
in this API.
