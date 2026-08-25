# Error Surface

Derived from every conditional fallback/rejection in `c_src/src/lib.c`, plus
the mandatory generic FFI null-pointer boundary cases. The C source contains
no `assert`, error enum, explicit numeric range check, length argument, or enum
argument.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| E01 | `parse_env_numeric` | named environment value contains `,` | return `default_val`; write `Warning: Invalid character in <name>\n` to stderr | [x] |
| E02 | `parse_env_numeric` | named environment value contains `;` and contains no `,` | return `default_val`; write `Warning: Semicolon found in <name>\n` to stderr | [x] |
| E03 | `envy` | computed result is negative after bit operations and adding `base_offset` | restore the copied state and return the original `param1` | [x] |
| E04 | `parse_env_numeric` | `env_name == NULL` | process terminates with `SIGSEGV` in the built C library | [x] |
| E05 | `init_config_from_env` | `flags == NULL` | process terminates with `SIGSEGV` in the built C library | [x] |
| E06 | `perform_operation` | `flags == NULL` | process terminates with `SIGSEGV` in the built C library | [x] |
| E07 | `apply_bit_operations` | `flags == NULL` | process terminates with `SIGSEGV` in the built C library | [x] |

Generic zero/oversized-length and out-of-range-enum cases are not applicable:
none of the exported functions accepts a length or enum argument. All scalar
arguments use the full C `int` type, with no documented narrower range.
