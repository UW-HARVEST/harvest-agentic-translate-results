# Error Surface

Mechanical scan:

```text
rg -n 'RETURN_ERROR|return\s+-1|return\s+NULL|assert\s*\(|NAN|NULL|abort|exit|if\s*\(|switch\s*\(' c_src
```

There are no pointer or length parameters, error enums, assertions, null
checks, min/max checks, or error-return macros in the public C API. Wrap
values and octave counts are not validated by C. A zero non-power-of-two wrap
is explicitly converted to 256. Non-positive octave counts execute zero loop
iterations and return `0.0f`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `inner` | `which` is not one of `0, 1, 2, 3, 4, 5` (the `switch` default branch) | `NAN` as a `float` | [x] |

The generic FFI boundary categories requested by Phase C do not add rows:
there are no pointers, buffers, lengths, or C enum parameters. `which` is the
only selector and row 1 covers its out-of-range values.
