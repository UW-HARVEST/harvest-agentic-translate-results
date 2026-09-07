# Configuration Surface

The crate declares no Cargo features, and CMake builds one shared library with
no source-level configuration macros. Rows below are the runtime branch and
input-shape combinations mechanically derived from `c_src/src/lib.c`.

For `buffapp`, `param1 % 4` and `param3 % 4` select one of five behavior
classes: `unknown` (negative remainder), `add` (`0`), `subtract` (`1`),
`multiply` (`2`), or `divide` (`3`). Each pair row uses randomized signed
operands; divide rows include zero and nonzero divisors, and rows exercise the
reachable zero/nonzero `intermediate3` final branch.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|-----|
| 1 | `create_buffer`, `destroy_buffer` | positive capacities: one byte, small, and larger buffers; initial length `0` and first byte NUL | [x] |
| 2 | `create_buffer` | zero capacity boundary (allocator-dependent C result/fields) | [x] |
| 3 | `create_buffer` | oversized positive capacity (`INT_MAX`) boundary | [x] |
| 4 | `create_buffer` | negative capacity (converted by C `malloc` to a huge `size_t`) | [x] |
| 5 | `append_to_buffer` | empty buffer + empty string; no growth | [x] |
| 6 | `append_to_buffer` | empty buffer + nonempty string; exact fit (`required_capacity == capacity`) | [x] |
| 7 | `append_to_buffer` | empty buffer + nonempty string; growth (`required_capacity > capacity`) | [x] |
| 8 | `append_to_buffer` | nonempty buffer + nonempty string; no growth | [x] |
| 9 | `append_to_buffer` | nonempty buffer + nonempty string; growth | [x] |
| 10 | `append_to_buffer` | repeated appends crossing multiple growth boundaries | [x] |
| 11 | `destroy_buffer` | normal object with non-null data | [x] |
| 12 | `destroy_buffer` | object with null data | [x] |
| 13 | `get_operation_name` | code `0` (`"add"`) | [x] |
| 14 | `get_operation_name` | code `1` (`"subtract"`) | [x] |
| 15 | `get_operation_name` | code `2` (`"multiply"`) | [x] |
| 16 | `get_operation_name` | code `3` (`"divide"`) | [x] |
| 17 | `get_operation_name` | all other integer classes, including negative and extreme values (`"unknown"`) | [x] |
| 18 | `perform_operation` | `"add"` with randomized negative/zero/positive operands and explicit wrapping-overflow boundaries | [x] |
| 19 | `perform_operation` | `"subtract"` with randomized negative/zero/positive operands and explicit wrapping-overflow boundaries | [x] |
| 20 | `perform_operation` | `"multiply"` with randomized negative/zero/positive operands and explicit wrapping-overflow boundaries | [x] |
| 21 | `perform_operation` | `"divide"` with randomized negative/positive divisors; the terminating `INT_MIN / -1` boundary is isolated in `ERRORS.md` | [x] |
| 22 | `perform_operation` | `"divide"` with zero divisor | [x] |
| 23 | `perform_operation` | unknown nonempty operation string | [x] |
| 24 | `perform_operation` | empty operation string | [x] |
| 25 | `buffapp` | stage 1 `unknown`; stage 2 `unknown` | [x] |
| 26 | `buffapp` | stage 1 `unknown`; stage 2 `add` | [x] |
| 27 | `buffapp` | stage 1 `unknown`; stage 2 `subtract` | [x] |
| 28 | `buffapp` | stage 1 `unknown`; stage 2 `multiply` | [x] |
| 29 | `buffapp` | stage 1 `unknown`; stage 2 `divide` | [x] |
| 30 | `buffapp` | stage 1 `add`; stage 2 `unknown` | [x] |
| 31 | `buffapp` | stage 1 `add`; stage 2 `add` | [x] |
| 32 | `buffapp` | stage 1 `add`; stage 2 `subtract` | [x] |
| 33 | `buffapp` | stage 1 `add`; stage 2 `multiply` | [x] |
| 34 | `buffapp` | stage 1 `add`; stage 2 `divide` | [x] |
| 35 | `buffapp` | stage 1 `subtract`; stage 2 `unknown` | [x] |
| 36 | `buffapp` | stage 1 `subtract`; stage 2 `add` | [x] |
| 37 | `buffapp` | stage 1 `subtract`; stage 2 `subtract` | [x] |
| 38 | `buffapp` | stage 1 `subtract`; stage 2 `multiply` | [x] |
| 39 | `buffapp` | stage 1 `subtract`; stage 2 `divide` | [x] |
| 40 | `buffapp` | stage 1 `multiply`; stage 2 `unknown` | [x] |
| 41 | `buffapp` | stage 1 `multiply`; stage 2 `add` | [x] |
| 42 | `buffapp` | stage 1 `multiply`; stage 2 `subtract` | [x] |
| 43 | `buffapp` | stage 1 `multiply`; stage 2 `multiply` | [x] |
| 44 | `buffapp` | stage 1 `multiply`; stage 2 `divide` | [x] |
| 45 | `buffapp` | stage 1 `divide`; stage 2 `unknown` | [x] |
| 46 | `buffapp` | stage 1 `divide`; stage 2 `add` | [x] |
| 47 | `buffapp` | stage 1 `divide`; stage 2 `subtract` | [x] |
| 48 | `buffapp` | stage 1 `divide`; stage 2 `multiply` | [x] |
| 49 | `buffapp` | stage 1 `divide`; stage 2 `divide` | [x] |
