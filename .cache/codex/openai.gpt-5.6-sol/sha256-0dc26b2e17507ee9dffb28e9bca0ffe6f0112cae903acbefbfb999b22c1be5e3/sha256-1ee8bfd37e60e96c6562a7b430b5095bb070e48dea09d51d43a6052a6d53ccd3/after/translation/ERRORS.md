# Error Surface

Derived from every rejecting branch in `../c_src/src/lib.c`. The allocation
failure is an explicit branch even though it requires process resource
exhaustion to trigger through the public API.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `parse_number` | `input_buffer == NULL` | returns `false` (`0`) | [x] |
| 2 | `parse_number` | `input_buffer->content == NULL` | returns `false` (`0`) | [x] |
| 3 | `parse_number` | `malloc(number_string_length + 1) == NULL` | returns `false` (`0`) without modifying `item` or `input_buffer->offset` | [x] |
| 4 | `parse_number` | `strtod` consumes no bytes (`number_c_string == after_end`), including zero available bytes or a scanned token with no valid numeric prefix | returns `false` (`0`) without modifying `item` or `input_buffer->offset` | [x] |

There are no `assert` statements, error enums, `RETURN_ERROR` macros, or
explicit input range rejection branches in the C source. `item == NULL` is not
rejected: after a successful conversion the C implementation dereferences it,
which is covered as a generic FFI boundary in the differential tests.
