# Error Surface

Mechanical source scan:

```text
rg -n 'RETURN_ERROR|return\s+(-1|NULL)|\bassert\s*\(|if\s*\(|switch\s*\(|NULL|enum' \
  ../c_src/include ../c_src/src
```

The scan has no matches. `driver` returns `void`, accepts one by-value `char`,
and contains no error return, assertion, range check, null check, enum, pointer,
or length. Consequently the C API has zero distinct rejection rows.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

Generic pointer, length, and enum boundary cases do not apply to this API.
Every bit pattern of its sole `char` argument is covered as valid input in
`CONFIGS.md`.

- [x] Phase C complete: zero C rejection paths and no inapplicable boundary
  categories were treated as test cases.
