# Error Surface

Mechanical audit inputs:

```text
rg -n 'RETURN_ERROR|return[[:space:]]+-1|return[[:space:]]+NULL|assert|enum|\
if[[:space:]]*\(|switch|case|NULL|MIN|MAX' c_src/include c_src/src
```

The only input-dependent condition is the successful loop termination check
`val % 10 == 9` in `sieve`; it does not reject input. The C API has no return
value, error enum, assertion, explicit range check, pointer, length, or enum
parameter. Consequently, the mechanically derived error-surface table has zero
rows.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

Generic boundary audit:

- Null pointers: inapplicable; `sieve` has no pointer parameters.
- Zero: valid scalar input and covered in `CONFIGS.md`.
- Zero/oversized lengths: inapplicable; `sieve` has no length parameters.
- Out-of-range enum values: inapplicable; `sieve` has no enum parameters.
- Documented range plus one: inapplicable; the header documents no restricted
  input range.

