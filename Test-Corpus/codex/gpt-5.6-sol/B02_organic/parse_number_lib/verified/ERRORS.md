# Error Surface

Derived from every rejecting branch in `c_src/src/lib.c` and the generic FFI
boundaries required by the verification protocol. There are no assertions,
error enums, public enum arguments, or documented numeric ranges that reject
an input.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|---|---|---|---|
| E01 | `parse_number` | `input_buffer == NULL` (`lib.c:23`) | returns `false` (`0`); `item` is untouched | [x] |
| E02 | `parse_number` | `input_buffer->content == NULL` (`lib.c:23`) | returns `false` (`0`); `item` and buffer are untouched | [x] |
| E03 | `parse_number` | `malloc(number_string_length + 1) == NULL` (`lib.c:64`) | returns `false` (`0`); `item` and buffer are untouched | [x] |
| E04 | `parse_number` | `strtod` consumes no byte, so `number_c_string == after_end` (`lib.c:85`); empty slice, zero length, or `offset == length` | frees the temporary allocation, returns `false` (`0`), and leaves `item` and `offset` unchanged | [x] |
| E05 | `parse_number` | scanner accepts bytes but `strtod` consumes none (`"."`, `"+"`, `"-"`, `"e"`, `"E"`, or combinations of only those bytes) | frees the temporary allocation, returns `false` (`0`), and leaves `item` and `offset` unchanged | [x] |
| E06 | `parse_number` | generic null-pointer boundary: `item == NULL` and the token converts successfully | C performs the unchecked write at `lib.c:92`; the isolated caller terminates with `SIGSEGV` | [x] |
| E07 | `parse_number` | generic oversized-length boundary: `length == SIZE_MAX`, `offset == 0`, and byte zero is an immediate non-number delimiter | scanner stops before advancing, `strtod` consumes no byte, returns `false` (`0`), and leaves state unchanged | [x] |

Allocation failure is tested with a process-local `malloc` interposer armed
immediately before each library call. The null-output boundary is tested in
isolated child processes because the C behavior is a terminating invalid
memory access.
