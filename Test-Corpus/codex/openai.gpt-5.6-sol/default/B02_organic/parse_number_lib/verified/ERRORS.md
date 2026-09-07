# Error Surface

Mechanically derived from every null check, allocation check, and failed
conversion branch in `../c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| 1 | `parse_number` | `input_buffer == NULL` | returns `false` (`0`); does not inspect or modify `item` | [x] |
| 2 | `parse_number` | `input_buffer->content == NULL` | returns `false` (`0`); does not inspect or modify `item` or the buffer | [x] |
| 3 | `parse_number` | `malloc(number_string_length + 1) == NULL` | returns `false` (`0`); does not modify `item` or `input_buffer->offset` | [x] |
| 4 | `parse_number` | `strtod` consumes zero bytes (`number_c_string == after_end`), including an empty available range or a scanned token with no numeric prefix | frees the temporary allocation, returns `false` (`0`), and does not modify `item` or `input_buffer->offset` | [x] |

Generic FFI boundaries are tested in addition to these explicit rejection
branches: null `item`, zero length, logical offsets at and above the declared
length (with the backing allocation kept valid), large safe lengths, and bytes
outside every C enum/type constant. This API has no enum-valued parameters.
