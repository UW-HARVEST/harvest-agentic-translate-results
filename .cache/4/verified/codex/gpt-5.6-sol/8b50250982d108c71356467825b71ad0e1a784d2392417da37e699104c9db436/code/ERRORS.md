# Error Surface

The C source was mechanically searched for rejection returns, null checks,
assertions, range checks, error enums, and min/max constants:

```text
rg 'RETURN_ERROR|return[[:space:]]+(-1|NULL)|assert|NULL|enum|MIN|MAX|if[[:space:]]*\(' c_src/include c_src/src
```

No public API rejection or error path exists. `get_predict_func` accepts the
full C `int` domain; values outside `0..=11` take the `default` switch arm and
return `0`.

| # | function | trigger (the exact invalid input/condition) | expected C result | Status |
|---|----------|---------------------------------------------|-------------------|--------|

Error-surface rows: **0**.

Generic pointer, length, and enum boundary cases are not applicable: the only
public API takes one `int`, with no pointer, length, or enum parameter.
