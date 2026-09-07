# Error Surface

Mechanical scans covered `c_src/include/` and `c_src/src/` for error-return
macros/statements, negative and null returns, assertions, conditionals,
switches, null checks, range comparisons, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|

There are zero source-defined rejection conditions. The exported functions
take one C `int` by value and return `void`; there are no pointer, length, enum,
option, mode, or documented-range inputs to reject.

Phase C status: [x] complete. Generic C-API boundary coverage is provided by
`phase_c_generic_integer_boundaries`, which compares `INT_MIN` and `INT_MAX`
through both exported entry points.
