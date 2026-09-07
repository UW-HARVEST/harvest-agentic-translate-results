# Error surface

Derived mechanically from every `return`, null check, regex compilation check,
and documented sentinel in `../c_src/src/lib.c`. There are no assertions,
enums, explicit numeric range checks, or min/max constants in this source.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `get_os_arch` | non-null string contains none of the 12 entries in `ARCHS` | returns `NULL` | [x] |
| 2 | `w_regexec` | `pattern == NULL` and `string != NULL` | returns `0` | [x] |
| 3 | `w_regexec` | `pattern != NULL` and `string == NULL` | returns `0` | [x] |
| 4 | `w_regexec` | `pattern == NULL` and `string == NULL` | returns `0` | [x] |
| 5 | `w_regexec` | both strings are non-null but `regcomp(..., REG_EXTENDED)` rejects `pattern` | writes the compile diagnostic to `stderr` and returns `0` | [x] |
| 6 | `parse_uname_string` | `osd == NULL` (including when `uname == NULL`) | returns immediately without dereferencing either pointer | [x] |

Unchecked pointer inputs are not rejection branches in C: `get_os_arch(NULL)`,
`parse_uname_string(NULL, non_null_osd)`, and a matching `w_regexec` call with
`nmatch > 0 && pmatch == NULL` enter libc with invalid pointers and terminate
by signal on this platform. Differential subprocess tests cover that crash
parity without pretending these are graceful errors.
