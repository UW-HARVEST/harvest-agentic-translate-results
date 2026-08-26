# ERRORS.md — error-surface table

Mechanically derived from `c_src/src/lib.c` (the only C source file). Every
rejection/error exit in the C code is listed below.

Grep of every `return` in the C source:

```
$ grep -n 'return\|assert\|if\s*(\|NULL\|-1\|<=\|>=' c_src/src/lib.c
6:char *custom_strdup(const char *str)
11:  if(!str)
12:    return (char *)NULL;
17:  if(!newstr)
18:    return (char *)NULL;
21:  return newstr;
```

There are exactly **two** failure branches (`if(!str)` and `if(!newstr)`), no
`assert`, no error enum, no `errno` write, no range check, no min/max constant
and no other rejection form anywhere in the library.

## Error-surface rows

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `custom_strdup` | `str == NULL` (`if(!str)`) — line 11-12 | returns `NULL` (`(char *)NULL`); nothing allocated, `errno` untouched | `err_e1_null_pointer`, `err_e1_null_repeated`, `err_e1_null_errno_untouched` | [x] |
| E2 | `custom_strdup` | `malloc(strlen(str)+1)` returns `NULL`, i.e. allocation failure (`if(!newstr)`) — line 17-18 | returns `NULL`; no copy performed, source untouched | `err_e2_malloc_failure_returns_null` (child process with `RLIMIT_AS` clamped so `malloc` of the duplicate fails) | [x] |

## Generic FFI boundary cases (covered even though not distinct C branches)

| # | case | expected behaviour (C ground truth) | test | status |
|---|------|-------------------------------------|------|--------|
| G1 | null pointer argument | `NULL` return (row E1) | `err_e1_null_pointer` | [x] |
| G2 | zero length input — `""` (the only "zero length" a NUL-terminated API can express) | returns a fresh 1-byte buffer holding `'\0'`; return value `!= NULL` and `!= str` | `err_g2_empty_string_is_not_an_error` | [x] |
| G3 | "oversized" length — huge input string (1 MiB, 4 MiB) | succeeds, full copy; no truncation, no error | `err_g3_oversized_input_succeeds` | [x] |
| G4 | one step past a valid range: `len` is `strlen+1`, so a string whose NUL is the very last readable byte of a mapped region (next page unmapped) | succeeds, copies exactly `strlen+1` bytes, never reads past the NUL (no fault) | `err_g4_no_read_past_terminator` | [x] |
| G5 | out-of-range "enum" values across the FFI boundary | the API has **no enum / flag / mode parameter at all** (single `const char *` argument), so there is no enum value to put out of range. The analogous "no valid variant" input for a pointer argument is a non-null but invalid pointer, whose behaviour is undefined in C and therefore intentionally not exercised; the representable invalid value `NULL` is row E1. | `err_g5_no_enum_parameters` (documents/asserts the single-arg ABI shape via both `.so`s) | [x] |
| G6 | misaligned / interior source pointer (offset 1..8 into a buffer) | succeeds identically | `err_g6_unaligned_source` | [x] |
| G7 | returned buffer must be `free()`-able with the C allocator and independent of the source | `free(p)` valid; mutating the copy leaves the source unchanged | `err_g7_result_is_independent_c_heap_block` | [x] |

All rows are covered by differential tests in `tests/differential.rs` that call
**both** the C `.so` and the Rust `.so` through `libloading` and assert the same
sentinel (`NULL` vs non-`NULL`) and the same bytes.
