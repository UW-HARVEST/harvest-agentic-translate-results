# Error Surface

Derived mechanically from every conditional return, switch default, null use,
assertion, explicit range check, and error-return pattern in
`../c_src/src/lib.c` and `../c_src/include/lib.h`.

The C source contains no `assert`, explicit null check, length parameter, enum
parameter, `RETURN_ERROR`, `return -1`, or `return NULL`.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `classify_mode` | non-null NUL-terminated string unequal to `"standard"`, `"enhanced"`, `"turbo"`, and `"extreme"` | returns `0x00` | [x] |
| 2 | `apply_multiplier` | `level < 0` or `level > 4` (the `default` switch arm) | returns `0xDEAD` | [x] |
| 3 | `classify_mode` | generic FFI null-pointer boundary: `mode == NULL`; C performs `strcmp(NULL, ...)` without a null check | process terminates with a fault signal | [x] |

Generic-boundary applicability:

- There are no length/count arguments, so zero/oversized lengths do not apply.
- There are no C enum arguments, so invalid enum discriminants do not apply.
- Zero and extreme integer values are exercised in `CONFIGS.md` and the
  differential suite.

