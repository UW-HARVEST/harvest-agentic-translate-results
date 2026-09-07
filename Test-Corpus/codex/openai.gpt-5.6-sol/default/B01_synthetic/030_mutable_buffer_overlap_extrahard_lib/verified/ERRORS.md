# Error Surface

Mechanical scan:

```text
rg -n 'RETURN_ERROR|return\s+-1|return\s+NULL|return\s+[A-Z_]*ERROR|assert\s*\(|if\s*\(|switch\s*\(|#ifdef|#if|NULL|MIN|MAX' ../c_src/include ../c_src/src
```

The scan found no input-rejection path in the C implementation. Both public
dynamic symbols return `void`; the source contains no error-return statement,
assertion, explicit null/range check, enum, or min/max input constant.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are therefore zero rejection rows to check off. Defined boundary
behavior that does not reject input is covered in `CONFIGS.md`: `fma_array`
with zero or negative `len` performs no accesses, and zero-length `driver` is
tested with an actual null data pointer against the built C library. Positive
lengths with null or undersized buffers and negative/huge VLA lengths in
`driver` invoke undefined behavior in the C source, so the C implementation
supplies no stable error result to compare. There are no enum parameters or
documented bounded integer ranges.

- [x] Every explicit C rejection path has a differential test (zero exist).
- [x] Applicable defined generic boundaries are covered.
