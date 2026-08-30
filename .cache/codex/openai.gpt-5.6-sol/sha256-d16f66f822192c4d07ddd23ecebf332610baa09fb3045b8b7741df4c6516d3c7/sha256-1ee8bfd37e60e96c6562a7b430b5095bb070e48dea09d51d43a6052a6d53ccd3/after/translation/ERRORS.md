# Error Surface

The public header declares only `void driver(float x)`. Mechanical searches of
`../c_src/include` and `../c_src/src` find no error-return macro or statement,
`assert`, error enum, explicit range check, null check, or min/max constant.
The API has no pointer, length, option, or enum argument. Every 32-bit object
representation is accepted as a `float`, including infinities, signed zeros,
subnormals, and all NaN payloads.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are no rejection rows to test. The generic pointer, length, and invalid
enum boundaries are not applicable to this scalar-only API.

- [x] Every rejection row has a passing differential test (empty set).
