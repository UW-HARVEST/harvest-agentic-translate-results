# Error Surface

Mechanical scans covered `c_src/include/` and `c_src/src/` for error-return
macros/statements, assertions, conditionals, switches, null checks, enums,
range checks, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|

There are no rejection paths: `helloworld` accepts no arguments, performs no
checks, and unconditionally returns `0`. Consequently, null pointers, lengths,
boundary values, and out-of-range enum inputs are not representable at this
API boundary.

Completion: [x] every C rejection row is covered (zero rows).
