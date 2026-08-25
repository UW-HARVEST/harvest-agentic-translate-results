# Error Surface

Derived from `c_src/src/lib.c` by enumerating every `NULL` return, error return,
assertion, explicit range check, and null check. The source contains no asserts,
error enums, explicit size range checks, or other error sentinels.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `encode_base64` | `src == NULL` at line 33 | returns `NULL` without dereferencing `src` | [x] |
| 2 | `encode_base64` | `calloc(1, size * 4 / 3 + 4) == NULL` at line 42 | returns `NULL` | [x] |

## FFI boundary cases

The C API has no enum parameters and documents no numeric maximum. Phase C
also exercises zero, negative, and large lengths. A non-null pointer that is not
readable for the selected positive length invokes undefined behavior in C and is
therefore not an input with a defined result to compare.
