# Error Surface

Mechanical searches covered `c_src/include/lib.h` and `c_src/src/lib.c` for
error returns, `NULL`, assertions, enums, range checks, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|

The C API contains no error-return path, assertion, null check, enum, pointer,
length, or explicit range rejection. `pow43` accepts one scalar `int`.
Phase C is complete with zero applicable error rows.

The table lookup is defined for the contiguous domain `-16 <= x <= 8223`.
Values outside that domain can index outside `g_pow43`; this is C undefined
behavior and does not produce a specified rejection result, so it is not an
error-surface row.
