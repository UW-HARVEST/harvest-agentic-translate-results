# Error Surface

The C source and public header were mechanically searched for error returns,
assertions, null checks, range checks, min/max constants, enums, and conditional
rejection branches:

```sh
rg -n 'RETURN_ERROR|return\s+(-1|NULL)|assert\s*\(|if\s*\(|switch\s*\(|NULL|MIN|MAX|enum' \
  ../c_src/src ../c_src/include
```

The search finds no rejection path. Both exported functions return `void`;
`perform_expensive_operations` takes no arguments, and `long_exec` accepts an
`unsigned int`, for which every ABI bit pattern is valid.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are zero error-surface rows to test or check off.

## Generic FFI boundaries

| boundary class | applicability |
|---|---|
| null pointers | Not applicable: neither function accepts a pointer. |
| zero lengths | Not applicable: neither function accepts a length. |
| oversized lengths | Not applicable: neither function accepts a length. |
| one past a valid range | Not representable: all `unsigned int` values are valid seeds. |
| out-of-range enums | Not applicable: the API has no enum parameter. |

Seed `0` and `UINT_MAX` are valid configurations and are covered in
`CONFIGS.md`.
