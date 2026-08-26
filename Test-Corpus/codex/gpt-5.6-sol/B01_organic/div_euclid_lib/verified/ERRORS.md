# Error Surface

Mechanically derived from all conditionals and returns in
`c_src/src/lib.c`. The public API has no pointers, lengths, or enum parameters,
so generic null-pointer, length, and out-of-range-enum cases do not apply.
Every `c_int` bit pattern is representable at the FFI boundary.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `div_euclid` | `v2 == 0` | returns `0` | [x] |
