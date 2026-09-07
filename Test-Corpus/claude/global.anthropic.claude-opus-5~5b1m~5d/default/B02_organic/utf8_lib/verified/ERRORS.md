# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection / error / assertion site in
`c_src/src/lib.c`. The complete list of such sites found by grepping the source:

```
$ grep -n 'assert\|return NULL\|== NULL\|!= NULL\|return -\|errno' c_src/src/lib.c
40:    assert(string != NULL);            # w_utf8_drop
60:    assert(string != NULL);            # w_utf8_filter
76:    if (copy == NULL) {  77: return NULL;   # malloc failure
101:   if (copy == NULL) { 102: return NULL;   # realloc failure
```

There are **no** error enums, no `RETURN_ERROR` macro, no `return -1`, no `errno`
use, and no explicit numeric range checks in this library. The only numeric
constant is `#define REPLACEMENT_INC 4096` (a growth step, not a limit). The
"range checks" of the library are the byte-range tests inside the `valid_1` ..
`valid_4` macros; because they *reject bytes* rather than *reject calls*, they are
enumerated as the boundary rows below (and exhaustively covered in Phase B /
`CONFIGS.md`).

## Rejection rows

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| E1 | `w_utf8_drop`   | `string == NULL` → `assert(string != NULL)` fails (`NDEBUG` is **not** defined by `CMakeLists.txt`, so the assert is live) | `__assert_fail("string != NULL", ...)` → `SIGABRT` (signal 6), process aborts, never returns | [x] |
| E2 | `w_utf8_filter` | `string == NULL` → `assert(string != NULL)` fails | `__assert_fail("string != NULL", ...)` → `SIGABRT` (signal 6), process aborts, never returns | [x] |
| E3 | `w_utf8_filter` | `malloc(strlen(string)+1)` returns `NULL` (only reachable when the input contains at least one invalid byte, i.e. after the `*valid == '\0'` early-return) | returns `NULL` | [x] |
| E4 | `w_utf8_filter` | `realloc(copy, size)` returns `NULL` while emitting a U+FFFD replacement (`replacement != 0` and `repl < 3`) | returns `NULL` (the old `copy` is leaked — reproduced) | [x] |
| E5 | `w_utf8_filter` | `strdup(string)` returns `NULL` (fully-valid input, allocation failure); the C does not check it and returns it verbatim | returns `NULL` | [x] |

## Generic FFI boundary rows (required even though not in the C's own checks)

| #   | function | trigger | expected C result | [x] |
|-----|----------|---------|-------------------|-----|
| E6  | both | empty string `""` (zero length) | `w_utf8_drop` → the same pointer (points at the terminating NUL); `w_utf8_filter` → `strdup("")`, i.e. a fresh 1-byte `""` | [x] |
| E7  | both | first byte is a bare continuation byte `0x80`..`0xBF` — matches no `valid_N` | `w_utf8_drop` returns the input pointer unchanged (offset 0); `w_utf8_filter` takes the non-`strdup` path | [x] |
| E8  | both | one step past the valid 2-byte lead range: `0xC0`, `0xC1` (overlong, excluded by `(x)[0] >= (char)0xC2`) | rejected as invalid at that byte | [x] |
| E9  | both | `0xC2`, `0xDF` (inclusive boundaries of the valid 2-byte lead range) with a valid continuation | accepted, advance 2 | [x] |
| E10 | both | one step past the valid 4-byte lead range: `0xF5`..`0xF7` (pass `&0xF8 == 0xF0` but fail `(unsigned char)x[0] <= 0xF4`) | rejected as invalid at that byte | [x] |
| E11 | both | lead bytes `0xF8`..`0xFF` (match no `valid_N` at all) | rejected as invalid at that byte | [x] |
| E12 | both | `0xE0` with continuation one step below the range: `0x80`..`0x9F` (overlong) | rejected | [x] |
| E13 | both | `0xE0 0xA0` (inclusive lower boundary) | accepted, advance 3 | [x] |
| E14 | both | `0xED` with continuation `0xA0`..`0xBF` (UTF-16 surrogate half U+D800..U+DFFF) | rejected | [x] |
| E15 | both | `0xED 0x9F` (inclusive upper boundary below the surrogates) | accepted, advance 3 | [x] |
| E16 | both | `0xF0` with continuation one step below the range: `0x80`..`0x8F` (overlong) | rejected | [x] |
| E17 | both | `0xF0 0x90` (inclusive lower boundary) | accepted, advance 4 | [x] |
| E18 | both | `0xF4` with continuation one step past the range: `0x90`..`0xBF` (> U+10FFFF) | rejected | [x] |
| E19 | both | `0xF4 0x8F` (inclusive upper boundary) | accepted, advance 4 | [x] |
| E20 | both | **truncated multi-byte sequence at end of string**: lead byte `0xC2`/`0xE1`/`0xF1` followed immediately by the NUL terminator, and `0xE1 0x80 NUL` / `0xF1 0x80 NUL` / `0xF1 0x80 0x80 NUL`. The NUL fails `(x)[n] & 0xC0 == 0x80`, so the macro short-circuits **before** reading past the terminator | rejected at the lead byte; **no read past the NUL** (verified by placing the string at the very end of an `mmap`ed page followed by an unmapped guard page) | [x] |
| E21 | `w_utf8_filter` | out-of-range value for the `bool` parameter passed across FFI (`_Bool` accepts any `int` byte at the ABI level): `2`, `3`, `0x7F`, `0x80`, `0xFF` | identical output to `replacement = 1` (the C tests the byte for non-zero); must match the C byte-for-byte for every one of these values | [x] |
| E22 | `w_utf8_filter` | oversized input: many thousands of invalid bytes, forcing repeated `realloc` growth (`repl` bookkeeping wrap-around path `repl -= 3`) | correct 3-byte U+FFFD per invalid byte, no truncation, no heap overflow | [x] |
| E23 | both | a NUL byte immediately following an invalid byte / interior NUL as sequence terminator | scanning stops at the NUL; bytes after it are never examined | [x] |
