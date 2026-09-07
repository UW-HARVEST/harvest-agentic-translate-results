# Error surface

Mechanical source scans covered `c_src/include/lib.h` and `c_src/src/lib.c`
for error-return macros/statements, negative or null returns, assertions,
explicit null/range checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|

There are no rejection branches in the C implementation. In particular,
`stbds_hash_bytes` performs no pointer/length validation and `siphash` returns
`void`. Invalid nonzero-length pointers therefore have C undefined behavior,
not an error result or sentinel.

Generic FFI boundary coverage, despite the empty rejection table:

| # | function | boundary | expected C result | verified |
|---|----------|----------|-------------------|----------|
| G1 | `stbds_hash_bytes` | null pointer with zero length | returns the zero-length hash | [x] |
| G2 | `stbds_hash_bytes` | zero length with a non-null pointer | returns the zero-length hash | [x] |
| G3 | `stbds_hash_bytes` | null pointer with nonzero length | process terminates from invalid memory access; compare subprocess outcomes | [x] |
| G4 | `stbds_hash_bytes` | null pointer with `SIZE_MAX` length | process terminates from invalid memory access; compare subprocess outcomes | [x] |

There are no enum parameters, documented bounded numeric ranges, or error
codes in the public C API, so out-of-range enum and one-past-range checks are
not applicable.
