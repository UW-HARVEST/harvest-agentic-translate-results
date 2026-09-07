# Error surface

Mechanically derived from every explicit rejection in `../c_src/src/lib.c`.
The source contains no `assert`, error macro, range check, null check, public
enum, or length argument.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `ima_parse` | `ima_btoh32(header->type)` is not the four-byte CAF file type (`caff` bytes in the input) | `-1` [x] |
| 2 | `ima_parse` | header type is valid and `ima_btoh16(header->version) != 1` | `-2` [x] |
| 3 | `ima_parse` | parsing reaches a `data` chunk after setting `desc` and `pakt`, and `ima_btoh32(desc->format_id)` is not the four-byte IMA4 format ID (`ima4` bytes in the input); the last preceding `desc` chunk is the one checked | `-3` [x] |

Additional ABI-boundary cases are tested separately because the C source does
not reject them: null `data` and null `info` dereference and terminate the
caller rather than returning an error. There are no public enums or explicit
lengths for zero/oversized/out-of-range tests.
