# Error Surface

The C source has no `RETURN_ERROR`, `return -1`, `return NULL`, error enum,
`assert`, explicit range check, or explicit null check. Its two default
branches return sentinels:

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `classify_mode` | `mode` is a valid NUL-terminated string unequal to `standard`, `enhanced`, `turbo`, and `extreme` | `0x00` | [x] |
| 2 | `apply_multiplier` | `level < 0` or `level > 4` (the `switch` default) | `0xDEAD` | [x] |

## Generic FFI Boundaries

| # | function | boundary | expected C result | tested |
|---|----------|----------|-------------------|--------|
| G1 | `classify_mode` | `mode == NULL`; C performs no null check and passes it to `strcmp` | process terminates with `SIGSEGV` | [x] |

No API accepts a length or enum. Therefore zero/oversized lengths and invalid
enum discriminants are not applicable. Rows 1-2 cover the only value-domain
defaults; `apply_multiplier` is tested at both `-1` and `5`.
