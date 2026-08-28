# Error Surface

Mechanically derived from the checks at `src/lib.c:42`, `src/lib.c:66`, and
`src/lib.c:84`, plus the mandatory generic FFI null-pointer boundary. The C API
has no length arguments, enums, or restricted integer ranges.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `cleanup` | Internal `strncmp(input_str, expected_str, strlen(expected_str)) != 0`; both strings are fixed to `"VALID"`, so this branch is not reachable from the four integer arguments | Return `0`; write `Input string validation failed.\n` to stdout; call `cleanup_resources(NULL)` | [x] |
| 2 | `cleanup` | `malloc(50)` returns `NULL` | Return the switch accumulation; write `Memory allocation failed.\n` to stdout; call `cleanup_resources(NULL)` | [x] |
| 3 | `print_result` | Generic FFI boundary: `label == NULL` | On the target GLIBC used by the C build, write `(null): <result>\n`; no return value | [x] |
| 4 | `cleanup_resources` | Generic FFI boundary and explicit null check: `dynamic_str == NULL` | No-op; no output; no return value | [x] |
