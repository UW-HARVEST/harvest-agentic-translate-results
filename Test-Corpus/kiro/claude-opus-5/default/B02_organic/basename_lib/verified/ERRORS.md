# ERRORS.md — Error-surface table

Mechanically derived. Every line of `c_src/src/lib.c` that could reject,
error, assert, or range-check was grepped:

```sh
grep -nE 'return|assert|NULL|-1|errno|ERROR|if *\(|switch|#if' c_src/src/lib.c c_src/include/lib.h
```

Result: the library has **no error-return channel at all**. There is no
`RETURN_ERROR` macro, no error enum, no `assert`, no `return -1`, no
`return NULL`, no `errno` write, no explicit range/null check, and no
min/max constant. `tool_basename` has exactly one `return` statement
(`return path;`) and it always returns a non-NULL pointer *into the caller's
buffer* whenever the input is a valid NUL-terminated string.

Consequently the "rejection surface" consists only of *implicit* / contractual
failures: inputs that violate the documented precondition. Each is listed as a
row, with the C behaviour that the Rust MUST reproduce bit-for-bit.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `tool_basename` | `path == NULL` | No null check exists. `strrchr(NULL, '/')` dereferences address 0 → **SIGSEGV** (fault, not an error code). Rust must fault identically, i.e. must NOT "helpfully" return NULL or a sentinel. | [x] |
| 2 | `tool_basename` | `path` points at an empty string `""` (length 0, valid but the degenerate minimum) | Neither separator found → both `s1` and `s2` NULL → falls through all three branches → returns `path` **unchanged** (same pointer value as the argument). | [x] |
| 3 | `tool_basename` | `path` ends in a separator, e.g. `"a/"` — the returned component is zero-length | Returns pointer to the NUL terminator, i.e. an **empty string**, not NULL and not `"a"`. Callers get `""`. | [x] |
| 4 | `tool_basename` | `path` is **all** separators, e.g. `"///"`, `"\\\\"`, `"/\\/\\"` | Last separator wins; returns pointer to the terminator → empty string. | [x] |
| 5 | `tool_basename` | `path` contains no separator at all, e.g. `"file.txt"` | Both `strrchr` calls return NULL; returns the **input pointer itself** (identity), not a copy. | [x] |
| 6 | `tool_basename` | `path` contains only `/` (no `\`) | `s1` non-NULL, `s2` NULL → 2nd branch → `s1 + 1`. | [x] |
| 7 | `tool_basename` | `path` contains only `\` (no `/`) | `s2` non-NULL, `s1` NULL → 3rd branch → `s2 + 1`. | [x] |
| 8 | `tool_basename` | `path` contains **both** separators, last `/` **after** last `\` (e.g. `"a\\b/c"`) | `s1 && s2` true, ternary `s1 > s2` true → `s1 + 1`. | [x] |
| 9 | `tool_basename` | `path` contains **both** separators, last `\` **after** last `/` (e.g. `"a/b\\c"`) | `s1 && s2` true, ternary `s1 > s2` false → `s2 + 1`. | [x] |
| 10 | `tool_basename` | `path` contains both separators **at the same relative order but adjacent**, e.g. `"a/\\b"` and `"a\\/b"` | Exercises the `>` comparison on pointers one byte apart; whichever separator is at the higher address wins. Note the C compares *pointers*, so this is address order, i.e. string order. | [x] |
| 11 | `tool_basename` | `path` whose only separator is the **first** byte, e.g. `"/x"`, `"\\x"` | Returns `path + 1`; no underflow guard is needed or present. | [x] |
| 12 | `tool_basename` | `path` containing embedded high-bit / non-ASCII bytes (`0x80`–`0xFF`) around the separators | `char` is **signed** on x86-64 Linux, so `strrchr`'s comparison is against a sign-extended `char`. Bytes ≥ 0x80 must NOT be mistaken for `/` (0x2F) or `\` (0x5C); result is unaffected by them. Rust's `c_char` is `i8`, matching. | [x] |
| 13 | `tool_basename` | `path` containing byte `0x00` early (so the "string" is shorter than the buffer) — trailing garbage after the NUL | `strrchr` stops at the NUL, so trailing bytes past the terminator are **ignored**, even if they contain separators. | [x] |
| 14 | `tool_basename` | very long `path` (e.g. 64 KiB) with separators near the end | No length limit / no `MAX_PATH` constant exists in the C. Must succeed, no truncation. | [x] |
| 15 | `tool_basename` | `path` where a separator is the **last byte before** a multi-separator run, e.g. `"a//b//"` | Only the *last* occurrence matters (`strrchr`, not `strchr`); earlier runs ignored. | [x] |

## Out-of-range enum values

The public API has **no enum, flag, mode, or `int` parameter** — the single
parameter is `char *`. There is therefore no out-of-range-enum row to write.
The pointer-validity rows (#1) and the byte-value rows (#12, #13) are the FFI
analogue: they cover every representable input the C accepts for its only
argument, including the ones with no "valid" meaning.

Row #1 (NULL) is verified by a **subprocess** differential test: both libraries
are called with NULL in a forked child and the resulting termination signal is
compared, since neither side can be expected to return.
