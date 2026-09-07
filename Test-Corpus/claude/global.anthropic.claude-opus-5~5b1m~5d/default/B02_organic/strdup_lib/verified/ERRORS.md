# ERRORS.md — Phase A: error / rejection surface table

Derived mechanically by grepping `c_src/src/lib.c` for every `return`, every
early-exit branch, every null check, every `assert`, and every range/size
constant. The whole library is 22 lines, so the enumeration is exhaustive.

```
$ grep -n 'return\|assert\|if(\|if (' c_src/src/lib.c
11:  if(!str)
12:    return (char *)NULL;
17:  if(!newstr)
18:    return (char *)NULL;
21:  return newstr;
```

There are exactly **two** rejection branches (`lib.c:11-12` and `lib.c:17-18`),
plus the implicit `size_t` arithmetic edge at `lib.c:14`. No `assert`, no error
enum, no `errno` set, no explicit min/max constants — the only sentinel is
`NULL`.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `custom_strdup` | `str == NULL` (`lib.c:11`, `!str` is true) | returns `NULL`; no allocation performed, `errno` untouched | [x] |
| 2 | `custom_strdup` | `malloc(len)` fails / returns `NULL` (`lib.c:17`, out-of-memory) | returns `NULL`; nothing copied, no leak | [x] |
| 3 | `custom_strdup` | `str` non-NULL but points at `""` (empty string) — the smallest valid length; `len == 1`, boundary of the `strlen(str)+1` computation (`lib.c:14`) | returns a non-NULL 1-byte buffer containing `'\0'` (NOT an error, but the zero-length boundary every C API must be probed at) | [x] |
| 4 | `custom_strdup` | `str` points into a buffer whose only NUL is at the very last readable byte (one step past would be out of range) | returns full copy incl. terminator; must not over- or under-read by one | [x] |

### Generic FFI boundary cases also covered in Phase C

These are not distinct C branches but are mandated boundary probes for any C
API. Each has a differential test in `tests/differential.rs`.

| # | condition | expected C result |
|---|-----------|-------------------|
| G1 | null pointer argument (same as row 1, asserted as a strict pointer-equality-to-NULL check on both `.so`s) | `NULL` from both | [x] |
| G2 | zero length input (empty string, row 3) | 1-byte `""` copy from both | [x] |
| G3 | oversized length: a very large (1 MiB, 4 MiB) NUL-terminated string | identical byte-for-byte copy from both | [x] |
| G4 | embedded NUL bytes: input `"ab\0cd"` — the C stops at the first NUL, so bytes after it must NOT be copied | both return `"ab"` (len 3 incl. terminator); byte at index 3 of the result is unspecified and is NOT compared | [x] |
| G5 | non-UTF-8 / arbitrary binary bytes `0x01..0xFF` before the terminator (Rust must not assume UTF-8 validity) | identical copy from both | [x] |
| G6 | out-of-range enum values across the FFI boundary | **N/A** — the API takes no enum, no flag, and no integer parameter; `const char *` is the only parameter, so there is no enum discriminant to fuzz. Documented here to record that the check was performed, not skipped. | [x] |
| G7 | misaligned / unaligned `char*` (pointer to a non-word-aligned offset inside a buffer) | identical copy from both | [x] |
| G8 | returned pointer must be `free()`-able by the caller (allocator parity) | no crash / no allocator mismatch for either `.so` | [x] |

All rows checked — see Phase C section of `tests/differential.rs`.
