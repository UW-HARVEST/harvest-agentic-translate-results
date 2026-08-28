# Error Surface

Mechanical scan inputs:

```text
rg -n 'RETURN_ERROR|return[[:space:]]+(-1|NULL)|assert[[:space:]]*\(|enum|if[[:space:]]*\(|switch[[:space:]]*\(|NULL|[Mm][Ii][Nn]|[Mm][Aa][Xx]|<[=]?|>[=]?|==' ../c_src/src ../c_src/include
```

The only matched runtime condition is `val % 10 == 9`, which terminates a
successful operation. The C API has no error return, sentinel, assertion,
explicit range check, pointer, length, enum, or rejection branch.

| # | function | trigger (the exact invalid input/condition) | expected C result | Verified |
|---|----------|----------------------------------------------|-------------------|----------|

Distinct C rejection paths: **0**

