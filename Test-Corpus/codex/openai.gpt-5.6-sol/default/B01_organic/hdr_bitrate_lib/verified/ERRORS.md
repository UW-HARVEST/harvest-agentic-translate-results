# Error Surface

Mechanical searches of `../c_src/include/lib.h` and `../c_src/src/lib.c` found
no error-return macros, error enums, `assert` calls, null checks, range checks,
conditionals, or rejection returns. The only return is the successful table
lookup in `hdr_bitrate`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are therefore zero explicit C rejection paths to check off.

## Caller-Contract Boundaries

These are not error paths: the C implementation performs unchecked pointer
dereferences and array indexing, so C specifies no result for them.

| Boundary | C behavior | Differential-test disposition |
|----------|------------|-------------------------------|
| `h == NULL` | Undefined behavior when evaluating `h[1]` | Isolated death test; both shared libraries must terminate by signal |
| Buffer shorter than 3 bytes | Undefined behavior when evaluating `h[1]` or `h[2]` | Cannot assert a C result |
| Layer bits `(h[1] >> 1) & 3 == 0` | Undefined behavior from row index `-1` | Cannot assert a C result |
| Bitrate nibble `h[2] >> 4 == 15` | Undefined behavior from column index `15` | Cannot assert a C result |
| Zero/oversized length | Not applicable; the API has no length parameter | No call to construct |
| Out-of-range enum | Not applicable; the API has no enum parameter | No call to construct |
