# Error Surface

Mechanically derived from every rejection branch in `../c_src/src/lib.c`.
There are no assertions, error enums, documented min/max constants, or enum
parameters in the public API.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `hex2bin` | A valid hex digit is reached while `bin_pos >= bin_maxlen` (line 31), including zero capacity or a prefix that has filled the output buffer. | Returns `-1`; resets the reported byte count internally to zero; if `hex_end_p` is non-null, stores the position where capacity was detected. [x] |
| 2 | `hex2bin` | Parsing stops with one unmatched high nibble, so `state != 0U` (line 43): odd digit count at end, before an invalid character, or before a non-ignorable separator. | Returns `-1`; backs `hex_pos` up to the unmatched nibble, resets the reported byte count internally to zero, and stores that nibble position when `hex_end_p` is non-null. [x] |
| 3 | `hex2bin` | Parsing does not consume all `hex_len` bytes (`hex_pos != hex_len`) and `hex_end_p == NULL` (line 52), such as an invalid character at a byte boundary. | Returns `-1` instead of returning the successfully decoded prefix. [x] |

## Generic FFI boundaries with no C rejection branch

The C implementation does not validate `bin` or `hex` before dereferencing
them. Tests cover both non-dereferencing null-pointer paths and, in isolated
subprocesses, verify that dereferencing null `bin` or `hex` terminates C and
Rust with the same process signal. `ignore == NULL` and `hex_end_p == NULL`
are normal public modes, not errors. There are no enum arguments, so no
out-of-range enum case exists. Oversized `hex_len` and `bin_maxlen` are covered
with an invalid first byte so no access occurs beyond the actual storage.
