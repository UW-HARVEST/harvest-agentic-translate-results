# ERRORS.md — Phase A: error-surface table

Mechanically derived from the *entire* C source. Grep results that establish
completeness:

```
$ grep -nE 'RETURN_ERROR|return -1|return NULL|assert|errno|goto|if *\(' c_src/src/lib.c
13:  if(s1 && s2) {
16:  else if(s1)
18:  else if(s2)
$ grep -c . c_src/include/lib.h          # 1 line: the prototype only
```

`tool_basename` is a **total function over its documented domain**: it contains
**no** error-return macro, **no** `return -1` / `return NULL`, **no** `assert`,
**no** `errno` use, **no** explicit range check, **no** null check, and **no**
min/max constant. There is no error enum and no out-parameter status. Every
input that is a valid NUL-terminated string produces a non-NULL pointer into
that same string.

Consequently the "error surface" consists solely of the *implicit* rejections /
undefined-behaviour boundaries that the C code reaches by not checking. Each is
listed as one row and each has a differential test.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `tool_basename` | `path == NULL`. C passes it straight to `strrchr(NULL, '/')` with no null check → dereference of address 0. | Process fault (`SIGSEGV`); no value returned. Rust must fault identically, not return a value or a different signal. | `err_01_null_pointer_faults_identically` (out-of-process, compares wait-status/signal of C vs Rust child) | [x] |
| 2 | `tool_basename` | Empty string `""` (length 0 — the "zero length" boundary). `strrchr` finds neither separator. | Returns `path` unchanged (pointer to the NUL byte itself), i.e. offset 0. | `err_02_empty_string` | [x] |
| 3 | `tool_basename` | String consisting of exactly one separator, `"/"` — separator is the last byte, so the result points one past it, at the NUL terminator. | Returns `path + 1`, a pointer to the terminating NUL (an *empty* basename, not an error). | `err_03_separator_only_returns_empty_tail` | [x] |
| 4 | `tool_basename` | String consisting of exactly one backslash, `"\\"` (the `s2`-only variant of row 3). | Returns `path + 1` → empty basename. | `err_03_separator_only_returns_empty_tail` | [x] |
| 5 | `tool_basename` | Trailing separator on a real path, `"a/b/"` — result is the empty string after the final `/`, so the "basename" is empty even though the input was not. | Returns pointer to the NUL byte (offset `strlen(path)`). | `err_04_trailing_separator` | [x] |
| 6 | `tool_basename` | Both separators present and **equal-position impossible**, but the `s1 > s2` tie-branch: `s1` and `s2` can never be equal (different bytes), so the `else` arm `s2 + 1` must be taken whenever `s2 > s1`. Constructed input `"a\\b/c"` vs `"a/b\\c"` covers both arms. | `(s1 > s2) ? s1+1 : s2+1` — the *later* separator wins. | `err_05_both_separators_ordering` | [x] |
| 7 | `tool_basename` | Oversized input: string much longer than any buffer heuristic (64 KiB, no separator, and 64 KiB with a separator at the very end). C has no length limit, so it must succeed. | Returns correct offset; no truncation, no overflow. | `err_06_oversized_length` | [x] |
| 8 | `tool_basename` | Bytes that are *not* valid UTF-8 (e.g. `0x80 0xFF 0xFE`) and high-bit bytes adjacent to separators. Naive Rust translations that go through `str`/`String` reject these; C does not care. | Treated as ordinary opaque bytes; separator search unaffected. | `err_07_non_utf8_bytes` | [x] |
| 9 | `tool_basename` | Byte values that are *one step past* the separators in the ASCII table and therefore must **not** match: `'.'`(0x2E) vs `'/'`(0x2F) vs `'0'`(0x30), and `'['`(0x5B) vs `'\\'`(0x5C) vs `']'`(0x5D). | No match → `path` returned unchanged. | `err_08_near_miss_separator_bytes` | [x] |
| 10 | `tool_basename` | Interior NUL: buffer `"a/b\0c/d"`. C's `strrchr` stops at the first NUL, so the bytes after it are invisible even though they contain a later `/`. | Searches only `"a/b"` → returns offset 2. The trailing `/d` is ignored. | `err_09_interior_nul_truncates_search` | [x] |
| 11 | `tool_basename` | Aliasing / mutation contract: the returned pointer must point **into the caller's own buffer** (no copy, no allocation the caller would have to free, input not modified). A Rust version returning a freshly allocated string would leak and break pointer arithmetic done by the caller. | `path <= ret <= path + strlen(path)`; input bytes unchanged. | `err_10_returns_interior_pointer_no_copy` | [x] |
| 12 | `tool_basename` | Out-of-range enum values passed across FFI. **N/A — no enum, no flag, no mode parameter exists in this API** (`grep -nE 'enum|typedef|#define' c_src/src/lib.c c_src/include/lib.h` → no match). Recorded so the class is explicitly discharged rather than silently skipped. | — | documented in `configs_and_errors_coverage` | [x] |

## Notes on row 1

Row 1 is the only genuinely fatal input. It cannot be asserted in-process
(the fault would kill the test runner), so the test re-executes the test
binary as a child process — once for the C `.so`, once for the Rust `.so` —
and compares the raw `wait` status, i.e. it asserts *the same signal*, not
merely "both failed somehow".
