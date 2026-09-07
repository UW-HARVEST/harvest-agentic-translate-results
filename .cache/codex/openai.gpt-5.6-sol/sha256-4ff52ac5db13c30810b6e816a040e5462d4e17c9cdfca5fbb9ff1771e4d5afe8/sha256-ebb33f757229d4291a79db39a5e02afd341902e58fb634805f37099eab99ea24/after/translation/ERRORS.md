# Error Surface

Mechanical source scan covered `RETURN_ERROR`, negative and null returns,
assertions, null checks, explicit range checks, min/max constants, and all
`default` branches in `../c_src/src/lib.c`.

The library has no error enum, pointer argument, length argument, assertion,
null check, or explicit min/max constant. Its sole public implementation has
one rejection branch:

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `call_predict` | `pfcn` is any integer not in `0..=11`; the outer `switch` takes `default` | returns `0` | [x] |

Generic FFI boundaries that are not applicable: null pointers, zero lengths,
oversized lengths, and enum discriminants. The ABI is `int call_predict(int)`
and contains no C enum.
