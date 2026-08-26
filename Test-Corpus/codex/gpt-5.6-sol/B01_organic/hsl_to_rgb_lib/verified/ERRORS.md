# Error Surface

The C source and public header were mechanically searched for error returns,
error macros, assertions, null checks, explicit range rejection, enums, and
min/max constants:

```text
rg -n 'RETURN_ERROR|return\s+(-1|NULL)|assert\s*\(|enum|NULL|MIN|MAX' \
  c_src/src c_src/include
```

The search has zero matches. `hsl_to_rgb` returns `void` and contains no
defined rejection or error path, so the error-surface table has zero rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|

## Generic Boundary Audit

| boundary | applicability in the C API | tested |
|----------|----------------------------|--------|
| null `src` or `dest` | No null check exists. C dereferences these pointers, so a null pointer violates the API's implicit memory contract. Subprocess probes verify that both shared libraries terminate with the same signal. | [x] |
| zero or oversized length | Not applicable: the API has no length parameter and always accesses exactly three `float` elements. | N/A |
| one-past-range value | Not applicable: the header documents no restricted numeric range and C accepts every `float` bit pattern when the pointers are valid. Out-of-conventional-range and non-finite values are covered as valid inputs in `CONFIGS.md`. | [x] |
| out-of-range enum | Not applicable: the API has no enum parameter. | N/A |

- [x] Every C rejection branch is represented (there are none).
- [x] Every applicable generic boundary with a defined C result is represented (there are none).
