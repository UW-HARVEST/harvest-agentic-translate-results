# Error Surface

Mechanically derived from every assignment to `ret = -1` and its guarding
condition in `../c_src/src/lib.c`. The C implementation has no assertions,
enums, error macros, or explicit null-pointer rejection branches.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| E01 | `hex2bin` | At a valid hex nibble, `bin_pos >= bin_maxlen` (including `bin_maxlen == 0`, or the first nibble after filling the output buffer) | `-1`; if `hex_end_p` is non-null it points at that nibble; already-written prefix bytes remain in `bin` | [x] |
| E02 | `hex2bin` | Parsing stops with `state != 0`, meaning one unmatched high nibble was consumed (end of `hex_len` or a non-hex/non-ignored byte while between nibbles) | `-1`; `hex_pos` is decremented so non-null `hex_end_p` points at the unmatched nibble | [x] |
| E03 | `hex2bin` | `hex_end_p == NULL` and parsing did not consume exactly `hex_len` (`hex_pos != hex_len`) | `-1` instead of a successful partial parse | [x] |

Generic FFI boundaries with no corresponding C rejection branch are covered
by tests: null pointers in combinations that C does not dereference, zero
lengths, `usize::MAX` lengths in short-circuiting safe cases, and one-step
capacity boundaries. There are no enum parameters to test. Passing a null
`bin` or `hex` when C would dereference it, or claiming a length beyond an
actual allocation while parsing continues, is undefined behavior rather than
an input rejection and is not assigned an expected return value.
