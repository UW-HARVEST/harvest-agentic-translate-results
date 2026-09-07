# Error surface

The C header and implementation expose only `float half2float(uint16_t h)`.
Mechanical inspection found no rejection paths: no error-return statements or
macros, assertions, null/range checks, error enums, pointers, lengths, or
documented validity limits. Every possible `uint16_t` value is valid.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

Generic FFI boundary cases involving null pointers, zero/oversized lengths, or
out-of-range enum values are not applicable because the API accepts one
by-value `uint16_t` and returns one `float`.
