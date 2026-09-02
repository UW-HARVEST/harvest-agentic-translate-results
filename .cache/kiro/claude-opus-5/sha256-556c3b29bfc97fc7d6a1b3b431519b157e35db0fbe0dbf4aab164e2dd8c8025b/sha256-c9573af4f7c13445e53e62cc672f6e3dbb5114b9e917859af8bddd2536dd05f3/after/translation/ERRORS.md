# ERRORS.md — Phase A error-surface table

## How this table was derived

Mechanical grep of the entire C source (`c_src/src/driver.c`,
`c_src/include/driver.h`) for every rejection construct:

```sh
grep -nE 'return|assert|NULL|errno|RETURN_ERROR|-1|if |switch|#if|MAX|MIN' \
     src/driver.c include/driver.h
```

Non-comment hits: only `#include <stdio.h>`, `#include <string.h>` and the
`DRIVER_H_` include guard. The complete body of the library is:

```c
void driver(const char *s1, const char *s2) {
    printf("%zu\n", strcspn(s1, s2));
}
```

So, factually: **the C library contains zero error-return statements, zero
`assert`s, zero explicit range checks, zero null checks, zero error enums, and
zero min/max constants.** `driver` returns `void`, so it has no error channel at
all, and it never inspects its arguments before handing them to `strcspn`.

That makes the real rejection surface the *implicit* one: the preconditions
`strcspn` imposes on its two arguments. Violating them is undefined behaviour in
C, and the way the as-compiled C library actually behaves is the ground truth
the Rust must reproduce. The rows below are those implicit rejections, each one
established empirically against the built `libdriver.so` (a `driver` call in a
forked child; `139` = killed by `SIGSEGV`).

Two structural facts about glibc's `strcspn` drive most rows, and both were
confirmed by probing the compiled `.so`:

* **R-ORDER**: the reject set `s2` is read *before* `s1` is touched at all
  (glibc's generic path tests `reject[0]`/`reject[1]` first; the x86-64 SSE4.2
  path tests `*a == 0` first).
* **R-WHOLE**: the *entire* reject set is consumed before `s1` is scanned, even
  when `s1[0]` matches `s2[0]` — glibc builds its full lookup table / SIMD mask
  from `s2` up front, so an unterminated `s2` faults even when a naive
  short-circuiting implementation would have returned early.

## The table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| E1 | `driver` | `s2 == NULL`, `s1` a valid non-empty string (`driver("abc", NULL)`) | SIGSEGV (exit 139), no output |
| E2 | `driver` | `s2 == NULL`, `s1` a valid **empty** string (`driver("", NULL)`) — per R-ORDER the empty `s1` does *not* save the call | SIGSEGV (exit 139), no output |
| E3 | `driver` | `s2` points at an unmapped (`PROT_NONE`) page, `s1` a valid empty string | SIGSEGV (exit 139), no output |
| E4 | `driver` | `s2` points at an unmapped page, `s1` a valid non-empty string | SIGSEGV (exit 139), no output |
| E5 | `driver` | `s1 == NULL`, `s2` a valid multi-character reject set (`driver(NULL, "abc")`) | SIGSEGV (exit 139), no output |
| E6 | `driver` | `s1 == NULL`, `s2` a valid **single**-character reject set (`driver(NULL, "a")`) — glibc's `reject[1]=='\0'` fast path | SIGSEGV (exit 139), no output |
| E7 | `driver` | `s1 == NULL`, `s2` a valid **empty** reject set (`driver(NULL, "")`) — glibc's `*a == 0` fast path, degenerates to `strlen(s1)` | SIGSEGV (exit 139), no output |
| E8 | `driver` | `s1 == NULL` **and** `s2 == NULL` | SIGSEGV (exit 139), no output |
| E9 | `driver` | `s1` points at an unmapped page, `s2` a valid non-empty reject set | SIGSEGV (exit 139), no output |
| E10 | `driver` | `s1` points at an unmapped page, `s2` a valid empty reject set | SIGSEGV (exit 139), no output |
| E11 | `driver` | `s1` valid, `s2` **unterminated**: reject bytes run straight into an unmapped page, and `s1[0]` matches `s2[0]` (a short-circuiting implementation would return `0` without faulting) — per R-WHOLE | SIGSEGV (exit 139), no output |
| E12 | `driver` | `s1` **unterminated**: `s1`'s bytes are all outside the reject set and run straight into an unmapped page, `s2` valid | SIGSEGV (exit 139), no output |
| E13 | `driver` | `s1` unterminated running into an unmapped page, `s2` the **empty** reject set (`strlen`-equivalent path) | SIGSEGV (exit 139), no output |

### Generic FFI boundary conditions (no dedicated C check exists, covered anyway)

`driver` takes no integer, length, enum or flag parameter — its signature is
`void driver(const char *, const char *)` — so there is no length argument to
pass as zero/oversized and **no enum parameter whose out-of-range integer value
could be smuggled across the FFI boundary**. The only out-of-domain values
reachable through this API are pointer values (rows E1–E13) and byte values
inside the two strings. The latter are *not* errors — every one of the 255
non-NUL byte values is a legal input — so they are covered as valid
configurations in `CONFIGS.md` (rows C13–C16, which specifically pin down the
signed-`char` sign-extension hazard for bytes `0x80..=0xFF`).

Zero length is likewise legal, not an error: an empty `s1` and/or an empty `s2`
are well-defined inputs, covered by `CONFIGS.md` rows C1–C4. Oversized length
has no representation in this API. "One step past a documented valid range" maps
onto the byte-value domain and the reject-set-size thresholds where glibc
switches implementation strategy (`CONFIGS.md` rows C6–C9).

## Status

| row | differential test | status |
|-----|-------------------|--------|
| E1 | `tests/error_paths.rs::e1_s2_null_s1_nonempty` | [x] passes |
| E2 | `tests/error_paths.rs::e2_s2_null_s1_empty` | [x] passes |
| E3 | `tests/error_paths.rs::e3_s2_unmapped_s1_empty` | [x] passes |
| E4 | `tests/error_paths.rs::e4_s2_unmapped_s1_nonempty` | [x] passes |
| E5 | `tests/error_paths.rs::e5_s1_null_s2_multi` | [x] passes |
| E6 | `tests/error_paths.rs::e6_s1_null_s2_single` | [x] passes |
| E7 | `tests/error_paths.rs::e7_s1_null_s2_empty` | [x] passes |
| E8 | `tests/error_paths.rs::e8_both_null` | [x] passes |
| E9 | `tests/error_paths.rs::e9_s1_unmapped_s2_nonempty` | [x] passes |
| E10 | `tests/error_paths.rs::e10_s1_unmapped_s2_empty` | [x] passes |
| E11 | `tests/error_paths.rs::e11_s2_unterminated_match_at_zero` | [x] passes |
| E12 | `tests/error_paths.rs::e12_s1_unterminated` | [x] passes |
| E13 | `tests/error_paths.rs::e13_s1_unterminated_empty_reject` | [x] passes |
