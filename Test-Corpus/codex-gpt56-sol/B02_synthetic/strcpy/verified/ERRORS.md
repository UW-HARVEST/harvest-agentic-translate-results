# Error Surface

Scope: every explicit rejection or error result in the public shared-library
translation unit, `c_src/src/lib.c`. The CMake-only CLI input failures in
`src/main.c` are outside the `.so` API and cannot be invoked through
`process_strings`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| [x] E01 | `process_strings` | `input == NULL`, before operation dispatch | `-1` |
| [x] E02 | `process_strings` | `operation == 0 && reference == NULL` | `-2` |
| [x] E03 | `process_strings` | `operation == 2 && reference == NULL` | `-2` |
| [x] E04 | `process_strings` | `operation == 4 && reference == NULL` | `-2` |
| [x] E05 | `process_strings` | `operation` is not one of `0..=4`, including `-1`, `5`, `INT_MIN`, and `INT_MAX` | `-3` |
| [x] E06 | `validate_token` via operation 0 | input differs from `reference`, `"VALID"`, and `"OK"` | `0` |
| [x] E07 | `parse_command` via operation 1 | input differs from all five commands and `"ADMIN"` | `-1` |
| [x] E08 | `compare_prefix` via operation 2 | exact flag clear and input does not begin with the complete reference string | `0` |
| [x] E09 | `compare_prefix` via operation 2 | exact flag set and input differs from the reference and all five generated suffix variations | `0` |
| [x] E10 | `find_delimiter` via operation 3 | `input_len == 0` | `-1` |
| [x] E11 | `find_delimiter` via operation 3 | delimiter is `'|'`, no delimiter occurs before NUL or `input_len`, and input is exactly `"NONE"` | `-2` |
| [x] E12 | `find_delimiter` via operation 3 | delimiter is `':'`, no delimiter occurs before NUL or `input_len`, and input is exactly `"EMPTY"` | `-3` |
| [x] E13 | `find_delimiter` via operation 3 | no delimiter occurs before NUL or `input_len` and neither special sentinel applies | `-1` |
| [x] E14 | `match_pattern` via operation 4 | case-sensitive flag set and input is neither exact, wildcard-form, nor containing the reference | `0` |
| [x] E15 | `match_pattern` via operation 4 | case-sensitive flag clear and input is neither exact, case-sensitive prefix, nor equal-length ASCII case-insensitive match | `0` |

No `assert`, error enum, `RETURN_ERROR`, explicit maximum, or explicit minimum
occurs in `lib.c`. `input_len` and `ref_len` are `size_t`; the library performs
no oversized-length rejection. Tests must allocate any claimed readable bytes
so that boundary calls do not introduce C undefined behavior.
