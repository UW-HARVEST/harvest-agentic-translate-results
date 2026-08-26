# Error Surface

The mechanical scan covered `c_src/src/driver.c` and
`c_src/include/driver.h` for error returns, `NULL`, assertions, range checks,
enums, and min/max constants. It found no rejection branches: neither exported
function validates its input or returns an error code.

The generic FFI boundary checks mandated for every API are tracked below.
They are not C rejection paths; a null pointer is passed directly to `strchr`,
so the C behavior is undefined. On the reference platform, each call terminates
the isolated subprocess with `SIGSEGV`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| E1 | `foo` | `in == NULL`, with `c == 'A'` | subprocess terminates with `SIGSEGV` on the reference platform | [x] |
| E2 | `driver` | `in == NULL` | subprocess terminates with `SIGSEGV` on the reference platform | [x] |

There are no length parameters, enum parameters, documented numeric ranges, or
error sentinels. Zero-length input is the valid empty C string and is covered
in `CONFIGS.md`; oversized lengths and out-of-range enum values do not apply.
