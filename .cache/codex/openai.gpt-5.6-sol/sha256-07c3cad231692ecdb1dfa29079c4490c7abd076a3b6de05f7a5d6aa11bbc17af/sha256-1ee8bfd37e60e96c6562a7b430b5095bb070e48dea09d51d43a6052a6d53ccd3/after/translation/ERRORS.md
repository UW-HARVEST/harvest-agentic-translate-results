# Error Surface

The complete source set (`include/lib.h` and `src/lib.c`) contains no error
return, assertion, null check, range rejection, error enum, pointer-bearing
public API, or length-bearing public API. The sole argument is a by-value
struct containing three `unsigned char` fields, so every possible FFI input is
valid.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|

Distinct C rejection paths: **0**.

Generic FFI error boundaries are inapplicable: `tritanopia` accepts no
pointers, lengths, or enums.
