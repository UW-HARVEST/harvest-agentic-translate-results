# Error Surface

Mechanically derived from all null checks, failed regex operations, and
sentinel returns in `c_src/src/lib.c`. There are no assertions, error enums,
range checks, or min/max constants in the C source.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| E01 | `get_os_arch` | `os_header` contains none of the 12 strings in `ARCHS` | returns `NULL` | [x] |
| E02 | `w_regexec` | `pattern == NULL` (with any `string`) | returns `0` before compiling | [x] |
| E03 | `w_regexec` | `string == NULL` (with non-null `pattern`) | returns `0` before compiling | [x] |
| E04 | `w_regexec` | `regcomp(pattern, REG_EXTENDED)` returns nonzero, e.g. an unclosed `(` | prints the compile diagnostic and returns `0` | [x] |
| E05 | `w_regexec` | pattern compiles but `regexec` returns nonzero because the string does not match | returns `0` | [x] |
| E06 | `parse_uname_string` | `osd == NULL` (including when `uname == NULL`) | returns immediately without reading or mutating `uname` | [x] |

Generic FFI boundaries additionally required by Phase C are exercised by the
tests: null `os_header`; null `pmatch` with zero and nonzero `nmatch`; zero and
oversized `nmatch`; null `uname`; empty strings; and both pointers null. The C
API has no enum parameters or documented integer ranges.
