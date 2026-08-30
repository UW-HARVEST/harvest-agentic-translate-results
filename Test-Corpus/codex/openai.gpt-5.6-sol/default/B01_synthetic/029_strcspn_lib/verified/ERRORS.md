# Error Surface

Mechanical searches of `src/driver.c` and `include/driver.h` found no error
returns, error macros, assertions, enums, range checks, null checks, length
arguments, or min/max constants. `driver` returns `void` and delegates directly
to `strcspn` and `printf`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

The API nevertheless has two generic pointer boundaries. C does not reject
either input: passing one to `strcspn` is undefined behavior. The differential
tests isolate these calls in child processes and compare the observed process
result on the test platform.

| # | function | generic boundary | observed C result | tested |
|---|----------|------------------|-------------------|:---:|
| B1 | `driver` | `s1 == NULL`, `s2` points to a valid C string | process termination result matches Rust | [x] |
| B2 | `driver` | `s1` points to a valid C string, `s2 == NULL` | process termination result matches Rust | [x] |

Zero and oversized explicit lengths and out-of-range enum values are not
applicable because the public API has neither lengths nor enums.
