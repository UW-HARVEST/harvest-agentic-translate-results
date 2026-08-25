# Error Surface

Mechanical scan:

```text
rg -n 'RETURN_ERROR|return[[:space:]]+-1|return[[:space:]]+NULL|assert|if[[:space:]]*\(|switch[[:space:]]*\(|#[[:space:]]*if|NULL|MIN|MAX|ERROR' c_src/include c_src/src
```

The scan finds no rejection macro, error/sentinel return, assertion, explicit
range check, null check, conditional, switch, or min/max constant. The sole
public API takes one `uint16_t` by value, so every value representable at its C
FFI boundary is valid. Pointer, length, and enum boundary cases do not exist.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are zero error-surface rows to test in Phase C.
