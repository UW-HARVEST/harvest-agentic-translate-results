# Error Surface

The rejection inventory was derived by scanning `../c_src/include/` and
`../c_src/src/` for returns, error macros, assertions, conditionals, null
checks, range checks, enums, and min/max constants:

```sh
rg -n 'RETURN_ERROR|return\s+(-1|NULL)|assert|if\s*\(|switch\s*\(|#ifdef|#if|MIN|MAX|enum|NULL' \
  ../c_src/include ../c_src/src
```

The only conditional operation is the exponent minimum used by the
calculation. The only return is the successful `float` result.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rejection rows: the C source has no error return, error enum,
assertion, explicit range rejection, null check, or min/max validity check.

## Generic FFI Boundaries

`ldexp_q2(float, int)` accepts two by-value scalars. It has no pointer, length,
or enum argument, so null pointers, zero/oversized lengths, and invalid enum
discriminants are not applicable. The source documents no restricted range
for either scalar. `float` bit-pattern classes and the full `int` boundary are
therefore valid-path inputs covered by `CONFIGS.md`, not rejection paths.

- [x] All source-derived rejection rows are covered (0 rows).
- [x] Applicable generic scalar boundaries pass through both shared objects in
      `scalar_ffi_boundaries_match`.
