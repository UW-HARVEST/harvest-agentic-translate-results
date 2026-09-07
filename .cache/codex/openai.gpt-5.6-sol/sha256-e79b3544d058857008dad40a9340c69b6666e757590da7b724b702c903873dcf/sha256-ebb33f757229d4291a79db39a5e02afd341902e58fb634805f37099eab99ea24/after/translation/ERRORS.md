# Error Surface

Mechanically derived from every rejecting branch in
`../c_src/src/slicing.c`. The C API defines no enums, explicit min/max
constants, assertions, or length arguments.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `slice` | `start_ptr != NULL` and, after C's signed-to-`size_t` conversion, `*start_ptr > strlen(mystr)`; this includes every negative `int` | prints `Error: start is off the end of the string!\n`; returns `1` | [x] |
| 2 | `slice` | start passed validation, `stop_ptr != NULL`, and, after C's signed-to-`size_t` conversion, `*stop_ptr > strlen(mystr)`; this includes every negative `int` | prints `Error: stop is off the end of the string!\n`; returns `1` | [x] |
| 3 | `slice` | start and stop passed their end checks, `stop_ptr != NULL`, and `*stop_ptr <= start` | prints `Error: stop must come after start!\n`; returns `1` | [x] |

## Generic FFI boundaries

These are tested in addition to the source-derived rows:

- `mystr == NULL`: the C source performs no null check and calls `strlen(NULL)`;
  compare C and Rust as isolated child processes so undefined behavior cannot
  kill the test runner.
- `start_ptr == NULL` and `stop_ptr == NULL`: these are valid, source-defined
  option states and are covered in `CONFIGS.md`.
- Zero length: represented by an empty NUL-terminated string and covered in
  `CONFIGS.md`.
- Oversized explicit length and out-of-range enum: not applicable; this API has
  neither a length parameter nor an enum parameter.
