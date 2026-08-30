# Error Surface

Mechanical scan:

```sh
rg -n 'RETURN_ERROR|return\s+-1|return\s+NULL|\bassert\s*\(|\breturn\b|\
\bif\s*\(|\bswitch\s*\(|#\s*ifdef|#\s*if|NULL|\b(MIN|MAX)\b|enum' \
  ../c_src/include ../c_src/src
```

The scan finds only the header include guard. `driver` returns `void` and the C
implementation contains no rejection statement, error enum, assertion, range
check, null check, min/max constant, pointer argument, length argument, or enum
argument.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|

There are zero C rejection paths to test. The generic FFI boundaries also do
not apply: the complete public API is `void driver(int floors)`, whose only
argument is a by-value C `int`. Its representable boundary values, `INT_MIN`
and `INT_MAX`, pass the valid-path differential test.

- [x] Every C rejection row has a passing differential test (zero rows).
- [x] Every applicable generic FFI boundary is covered.
