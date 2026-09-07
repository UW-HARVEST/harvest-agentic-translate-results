# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from the complete C source (`c_src/src/driver.c`,
`c_src/include/driver.h`). The grep for every rejection construct:

```
$ grep -nE 'return|assert|NULL|errno|-1|#if|#ifdef|switch|\bif\b|\bwhile\b|\bfor\b|MAX|MIN|exit|abort' \
       c_src/src/driver.c c_src/include/driver.h
include/driver.h:24:#ifndef DRIVER_H_        <- include guard only
```

The entire body of the library is:

```c
void driver(const char *s1, const char *s2) {
    printf("%zu\n", strcspn(s1, s2));
}
```

So, mechanically: **zero** `RETURN_ERROR`-style macros, **zero** `return -1` /
`return NULL`, **zero** error enums, **zero** `assert`, **zero** explicit range
checks, **zero** null checks, **zero** min/max constants, and **zero**
`if`/`switch`/`#ifdef` branches. `driver` returns `void`, so it has no error
channel at all: there is no value it can return to reject an input.

The rejection surface is therefore entirely *implicit* — it consists of the
preconditions `strcspn` imposes on its arguments. Those rows are below, together
with the generic C-API boundaries the task requires be covered even when they are
not in the source. Rows marked *(UB)* are undefined behaviour in C; the
requirement there is that the Rust behaves the same way the C does, which is
verified out-of-process (fork + `waitpid`) so a matching crash is observable
instead of killing the harness.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| 1 | `driver` | `s1 == NULL`, `s2` a valid string *(UB)* | no output; process dies on a fatal signal (`SIGSEGV`) when dereferencing the null `s1` | `err_01_null_s1` | [x] |
| 2 | `driver` | `s1` a valid string, `s2 == NULL` *(UB)* | no output; process dies on a fatal signal (`SIGSEGV`) when dereferencing the null `s2` — but only if `s1` is non-empty, since `s2` is not read before the first byte of `s1` | `err_02_null_s2` | [x] |
| 3 | `driver` | `s1 == NULL` **and** `s2 == NULL` *(UB)* | no output; fatal signal (`SIGSEGV`) | `err_03_null_both` | [x] |
| 4 | `driver` | `s1 == ""` (empty) **and** `s2 == NULL` *(UB)* | no output; fatal signal (`SIGSEGV`). **Measured, not assumed:** glibc's `strcspn` materialises a membership table by walking the whole reject set *before* it reads any byte of `s1`, so the immediately-terminated `s1` never gets to short-circuit and the NULL `s2` is dereferenced regardless. This row initially diverged — the Rust `strcspn` checked `*s1` first and returned `0` — and `src/lib.rs` was changed to walk `s2` first to match the C. | `err_04_empty_s1_null_s2` | [x] |
| 5 | `driver` | `s1` NOT NUL-terminated (zero-length buffer, no terminator) *(UB)* | reads past the end of the buffer; result is whatever follows in memory — C and Rust must agree given identical adjacent bytes | `err_05_unterminated_s1` | [x] |
| 6 | `driver` | `s2` NOT NUL-terminated *(UB)* | reads past the end of the reject set; C and Rust must agree given identical adjacent bytes | `err_06_unterminated_s2` | [x] |
| 7 | `driver` | zero-length input: `s1 == ""`, any valid `s2` | prints `0\n` (no error — `strcspn("", x) == 0`) | `err_07_zero_len_s1` | [x] |
| 8 | `driver` | zero-length reject set: `s2 == ""`, any valid `s1` | prints `strlen(s1)` — the NUL of `s2` is *not* a member of the reject set, so nothing ever matches | `err_08_zero_len_s2` | [x] |
| 9 | `driver` | both empty: `s1 == ""`, `s2 == ""` | prints `0\n` | `err_09_both_empty` | [x] |
| 10 | `driver` | oversized length: `s1` far larger than any internal buffer (1 MiB, no match) | prints `1048576\n`; no truncation, no overflow, no error | `err_10_oversized_no_match` | [x] |
| 11 | `driver` | value one step past the valid byte range: `s1`/`s2` contain `0x80`–`0xFF` (bytes that are negative when `char` is signed, as it is on x86-64) | matched by value like any other byte; the sign of `char` must not change the comparison | `err_11_high_bit_bytes` | [x] |
| 12 | `driver` | embedded NUL: `s1 = "ab\0cd"`, `s2 = "d"` | scanning stops at the embedded NUL, so `d` past it is invisible; prints `2\n` | `err_12_embedded_nul_s1` | [x] |
| 13 | `driver` | embedded NUL in the reject set: `s2 = "z\0a"`, `s1 = "abc"` | only `z` is in the reject set (`a` is past the NUL); prints `3\n` | `err_13_embedded_nul_s2` | [x] |
| 14 | `driver` | out-of-range enum value across the FFI boundary | **not applicable**: `driver` takes no enum, no int, no flag parameter — the signature is `(const char*, const char*)` and there is no enum anywhere in the public header. Asserted structurally rather than at runtime: the header declares no enum type. | `err_14_no_enum_in_api` | [x] |
| 15 | `driver` | misaligned / non-`NULL` but invalid pointer (`(char*)1`) *(UB)* | fatal signal (`SIGSEGV`) | `err_15_wild_pointer` | [x] |
| 16 | `driver` | `s1` valid, `s2` = 1 MiB reject set containing every byte 0x01–0xFF | first byte of `s1` matches; prints `0\n` | `err_16_oversized_s2` | [x] |
