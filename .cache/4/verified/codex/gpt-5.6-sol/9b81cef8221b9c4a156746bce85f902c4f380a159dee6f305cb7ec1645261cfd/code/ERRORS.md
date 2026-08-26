# Error Surface

Mechanical searches covered `RETURN_ERROR`, negative and null returns,
`assert`, conditionals, switches, null checks, range checks, enums, and
minimum/maximum constants in `c_src/src/` and `c_src/include/`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|

There are **0 explicit rejection paths**. `driver` returns `void` and performs
no validation before passing both pointers to `strcspn`.

Null pointers are outside the C API contract because `strcspn` requires valid
null-terminated strings. The C source does not handle them or produce an error
sentinel. Length, enum, numeric range, and allocation boundaries do not apply:
the API has no length, enum, numeric, or caller-allocated output parameters.

## Generic Boundary Coverage

| boundary | applicability and differential coverage | status |
|----------|------------------------------------------|--------|
| null `s1` | outside the contract; C and Rust subprocesses terminate identically | [x] |
| null `s2` | outside the contract; C and Rust subprocesses terminate identically | [x] |
| zero length | no length parameter; empty `s1` and `s2` are valid and covered in `CONFIGS.md` | [x] |
| oversized length | no length parameter | N/A |
| one past valid numeric range | no numeric parameter or documented numeric range | N/A |
| out-of-range enum | no enum parameter | N/A |
