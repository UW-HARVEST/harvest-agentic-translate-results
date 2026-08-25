# Error Surface

Mechanical audit command:

```text
rg -n 'RETURN_ERROR|return\s+-1|return\s+NULL|assert\s*\(|if\s*\(|switch\s*\(|#\s*if|NULL|enum|MIN|MAX' c_src/src/main.c
```

The audit returns no matches. The C source has no rejection branches, error
macros, assertions, explicit null/range checks, enums, or min/max constants.
`main` ignores the return value from `scanf`, so failed conversion is not
returned as an error: the initialized value `0` is printed and `main` returns
`0`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

## Generic FFI Boundaries

The exported API has no pointer, length, or enum parameters. Consequently,
null pointers, zero/oversized lengths, and out-of-range enum discriminants do
not exist at this FFI boundary. The applicable generic input boundaries for
`main` are tracked in `CONFIGS.md` because C converts them to ordinary output
rather than rejecting them.

- [x] Zero explicit rejection rows remain to test.
- [x] Applicable generic boundaries pass differential tests.
