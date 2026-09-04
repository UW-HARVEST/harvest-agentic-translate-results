# Error surface

Mechanical searches covered every `return`, `if`, assertion spelling, null
constant, error macro spelling, and min/max/threshold spelling in
`../c_src/src/lib.c` and `../c_src/include/lib.h`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are **no explicit input rejections** in the C source: no error-return
macro, `return -1`, `return NULL`, error enum, `assert`, null-pointer check, or
invalid-range return.

The following branches are not rejections and therefore belong to the valid
configuration surface:

- `divide_multiplier`: `b == 0` skips division, increments
  `operation_count`, and returns the unchanged multiplier.
- `validate_and_normalize`: positive values below octal `0100` return `0100`;
  positive values above octal `0777` return `0777`.

Generic FFI boundaries not represented by a C rejection row:

- Null `dest` for `process_octal_string` and null `str` for
  `find_and_replace_char` are unchecked C undefined behavior. Differential
  tests invoke them in child processes and compare process termination:
  **passed for C and Rust**.
- No public function accepts an explicit length or enum parameter, so
  oversized-length and out-of-range-enum rejection cases do not exist.
- Zero integer arguments are valid inputs and are covered in `CONFIGS.md`.
- Long but valid NUL-terminated strings are covered in `CONFIGS.md`: **passed**.
