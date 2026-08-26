# Error Surface

Derived from every assignment or branch that makes `hex2bin` return `-1` in
`c_src/src/lib.c`. The C source contains no assertions, enums, error macros, or
other error sentinels.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| E01 | `hex2bin` | At line 31, a valid hex nibble is reached while `bin_pos >= bin_maxlen` (including non-empty input with `bin_maxlen == 0`, or input exceeding a nonzero output capacity). | `-1`; final logical output length is discarded; `hex_end_p`, when non-null, points at the unconsumed nibble. | [x] |
| E02 | `hex2bin` | At line 43, parsing stops or reaches `hex_len` with `state != 0`, meaning one valid high nibble has no valid low nibble. An ignore character is not accepted in this state. | `-1`; final logical output length is discarded; `hex_end_p`, when non-null, points at the unmatched high nibble. | [x] |
| E03 | `hex2bin` | At line 52, parsing stopped before `hex_len` and `hex_end_p == NULL` (invalid byte at a byte boundary, with `ignore == NULL` or the byte absent from `ignore`). | `-1`; already-written output bytes remain in memory, but no output length is returned. | [x] |

## FFI Boundaries

The C API has no explicit pointer validation and therefore does not define
behavior for a null pointer that is dereferenced. Differential tests cover all
safe null cases: null `hex` with zero length, null `bin` when no write can
occur, null `ignore`, and null `hex_end_p`. They also cover zero capacity,
capacity one byte below required, exact capacity, excess capacity, zero length,
and large allocated input lengths. There are no enum or documented scalar
range inputs in this API.
