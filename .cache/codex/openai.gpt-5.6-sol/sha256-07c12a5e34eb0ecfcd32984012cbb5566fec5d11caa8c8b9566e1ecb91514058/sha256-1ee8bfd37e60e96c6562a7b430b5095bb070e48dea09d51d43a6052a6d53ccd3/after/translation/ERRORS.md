# Error Surface

The C header and implementation were mechanically searched for error returns,
null checks, assertions, range checks, error enums, and min/max constants:

```text
rg -n 'RETURN_ERROR|return[[:space:]]+-1|return[[:space:]]+NULL|assert|NULL|ERROR|MIN|MAX|if[[:space:]]*\(|switch[[:space:]]*\(' \
  ../c_src/include ../c_src/src
```

No rejection or error path exists. `float2half` accepts one by-value `float`,
uses all 32 object-representation bits as a table index and mantissa, and
returns a `uint16_t` for every possible input bit pattern. Pointer, length,
enum, option, and state boundaries do not apply to this API.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are zero rows to check in Phase C.

