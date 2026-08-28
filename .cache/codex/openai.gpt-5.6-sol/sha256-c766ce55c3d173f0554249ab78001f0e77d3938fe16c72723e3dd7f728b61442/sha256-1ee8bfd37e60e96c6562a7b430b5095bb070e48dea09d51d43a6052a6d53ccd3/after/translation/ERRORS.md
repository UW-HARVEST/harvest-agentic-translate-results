# Error Surface

This table is derived from every C `return`, null check, regex compilation
failure, and documented sentinel in `src/lib.c`. The C source contains no
assertions, error enums, `return -1`, min/max constants, or explicit range
checks.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| E01 | `get_os_arch` | `os_header` contains none of the 12 strings in `ARCHS` | Returns `NULL` | [x] |
| E02 | `w_regexec` | `pattern == NULL` | Returns `0` without compiling or matching | [x] |
| E03 | `w_regexec` | `string == NULL` with non-null `pattern` | Returns `0` without compiling or matching | [x] |
| E04 | `w_regexec` | `pattern` is not a compilable POSIX extended regular expression | Writes the compile diagnostic to `stderr` and returns `0` | [x] |
| E05 | `w_regexec` | Valid compiled pattern does not match `string` | Returns `0` | [x] |
| E06 | `parse_uname_string` | `osd == NULL` | Returns immediately without reading or modifying `uname` | [x] |

Additional pointer/boundary behavior compared by
`phase_c_generic_pointer_and_length_boundaries`:

- [x] `get_os_arch(NULL)` is not rejected by C; C passes it to `strstr`.
- [x] `parse_uname_string(NULL, non_null_osd)` is not rejected by C; C passes it
  to `strstr`.
- [x] `w_regexec` accepts `nmatch == 0` with `pmatch == NULL`.
- [x] `w_regexec` does not validate `pmatch` when `nmatch > 0`; libc `regexec`
  defines the resulting memory access behavior.
- [x] Empty and 65,536-byte inputs match through all applicable entry points.
- [x] `nmatch == usize::MAX` has matching process termination behavior.
- [x] There are no enum parameters or documented numeric ranges in this API.
