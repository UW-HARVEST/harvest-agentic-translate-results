# Error Surface

Mechanical source scan:

```text
rg -n 'RETURN_ERROR|return[[:space:]]+-1|return[[:space:]]+NULL|assert|\
if|switch|MIN|MAX|NULL|scanf' c_src/src
```

The source has no error-return macro, error enum, assertion, explicit range
check, pointer argument, enum argument, or length argument. `main` ignores
`scanf`'s conversion count and always returns `0`. The only input rejections
are therefore the four distinct failure states of the two `%d` conversions.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `main` / first `%d` | EOF before the first conversion (empty input or whitespace followed by EOF) | `scanf` returns EOF; `x=0`, `y=0`; no stdout; `main` returns `0` | [x] |
| 2 | `main` / first `%d` | First non-whitespace byte cannot begin a decimal integer | `scanf` returns `0`; `x=0`, `y=0`; no stdout; `main` returns `0` | [x] |
| 3 | `main` / second `%d` | First conversion succeeds, then EOF occurs before the second conversion | `scanf` returns `1`; `x` has the first value, `y=0`; output follows the `y==0` path; `main` returns `0` | [x] |
| 4 | `main` / second `%d` | First conversion succeeds, then the next non-whitespace byte cannot begin a decimal integer | `scanf` returns `1`; `x` has the first value, `y=0`; output follows the `y==0` path; `main` returns `0` | [x] |

Generic FFI boundaries are not applicable: the sole public entry point is
`int main(void)` and accepts no pointers, lengths, or enum values. Zero integer
values are valid and are covered in `CONFIGS.md`. Decimal values outside C
`int` range are excluded because C leaves conversion overflow undefined.
