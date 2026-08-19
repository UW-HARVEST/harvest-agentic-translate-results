# ERRORS.md — Phase C: error-surface table

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Mechanical grep result (the anti-blind-spot step)

```
grep -nE 'return|assert|NULL|if|else|switch|#if|ERROR|errno|==|!=' c_src/src/driver.c c_src/include/driver.h
```

yields, excluding the licence comment block and `#include`s:

* `src/driver.c:34: return res;`  ← the **only** `return` statement
* `include/driver.h:24-29:` the `DRIVER_H_` include guard

That is the complete list. Therefore:

* there are **no** error-return macros (`RETURN_ERROR`, …), **no** error enums,
  **no** `return -1` / `return NULL`, **no** out-params carrying status;
* there are **no** `assert`s (`grep -c assert` = 0 in both files);
* there are **no** explicit range checks, no NULL checks, no min/max constants;
* `foo` returns `int` and can only return a non-negative count; `driver`
  returns `void` and can only fail by crashing.

The C library consequently has an *implicit* error surface only: invalid input
is not rejected, it is dereferenced. Every row below is a real input an external
caller can supply, and the Rust `.so` must reproduce the C `.so`'s reaction
exactly (same signal / same sentinel value), which is what the Phase C tests
assert. Rows 1–6 are memory-fault rows; they are checked by forking a child
process (re-exec of the test binary) and comparing the child's termination
signal for the C `.so` against the Rust `.so`.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `foo` | `in == NULL`, `c = 'A'` (non-zero). No NULL check exists, so `strchr(NULL,'A')` loads from address 0 on the very first iteration. | fatal `SIGSEGV`, no value returned | [x] |
| 2 | `foo` | `in == NULL`, `c = '\0'`. Same first-iteration NULL load (`c == 0` does not short-circuit anything). | fatal `SIGSEGV` | [x] |
| 3 | `foo` | `in` valid & NUL-terminated, **`c == '\0'`**. `strchr(s,0)` *always* succeeds (it returns the terminator), so the `for` condition is never NULL: `res` keeps incrementing and `s` walks past the end of the object forever. Placed at the end of an `mmap`ed page whose successor page is unmapped, this faults deterministically. | fatal `SIGSEGV` (loop never terminates normally) | [x] |
| 4 | `foo` | `in` points at a buffer with **no NUL terminator**, `c` present in the buffer. `strchr` keeps finding matches and then scans off the end of the object. Guard page ⇒ deterministic fault. | fatal `SIGSEGV` | [x] |
| 5 | `foo` | `in` points at a buffer with **no NUL terminator**, `c` absent from the buffer. `strchr` scans past the end looking for `c`-or-NUL. Guard page ⇒ deterministic fault. | fatal `SIGSEGV` | [x] |
| 6 | `driver` | `in == NULL`. `driver` forwards to `foo(in,'A')` before any `printf`, so it faults with **no output produced at all** (not even a partial `"A: "`). | fatal `SIGSEGV`, zero bytes written to stdout | [x] |
| 7 | `foo` | zero length: `in = ""` (points at a lone NUL), `c != 0`. First `strchr` returns NULL immediately. | returns `0` (sentinel-free success, *not* an error) | [x] |
| 8 | `foo` | `in` = one-past-the-last-character pointer of a longer buffer (i.e. aimed at that buffer's NUL), `c != 0`. | returns `0` | [x] |
| 9 | `foo` | `c` **out of range for `char`**: caller passes a full `int` with garbage in the upper 24 bits (`0x141`, `0xFFFFFF41`, `0x100`, `-1`, `INT_MIN`, `INT_MAX`). C performs no validation; the SysV ABI only defines the low 8 bits of the argument register, and the C callee reads exactly that byte. `0x141` must therefore behave like `'A'`. | no error; result equals `foo(in, (char)(v & 0xFF))` | [x] |
| 10 | `foo` | `c` = a byte with the high bit set (`0x80`–`0xFF`, i.e. a *negative* `char` on x86-64 where `char` is signed) matched against haystack bytes ≥ `0x80`. C promotes `char`→`int` (sign-extending) and `strchr` converts back to `unsigned char`, so `0xE9` matches the byte `0xE9`. A translation that compared as `u8` vs `i8` inconsistently would diverge here. | no error; correct occurrence count | [x] |
| 11 | `foo` | oversized length: 1 MiB haystack whose match count (≈ 524 288) is far larger than any buffer the caller sized. There is no length parameter and no cap, so nothing is rejected. | no error; full count returned in `int` | [x] |
| 12 | `driver` | oversized/one-step-past output width: input engineered so a count reaches 5+ decimal digits, i.e. past the width of every literal in the format string. `printf("%d")` is unbounded, so no truncation and no error. | no error; full decimal number printed | [x] |
| 13 | `foo` | `in` valid, `c` = a byte that is present in the haystack **only after an embedded NUL**. The embedded NUL terminates the scan, so those later matches are invisible — the C never looks past it and reports a *smaller* count than the buffer contains. | no error; count of matches strictly before the first NUL | [x] |

All 13 rows have a passing differential test; see `tests/differential.rs`
(`phase_c_*`). The generic boundaries requested in addition to the table —
NULL pointers (rows 1, 2, 6), zero length (row 7), oversized length (rows 11,
12), one step past a documented range (rows 8, 9) and out-of-range
`enum`-style values crossing the FFI boundary (row 9, where the `char`
parameter is deliberately called through an `extern "C" fn(*const c_char,
c_int)` prototype so values with no valid `char` representation really do reach
the callee) — are all covered by those rows. The C library declares no `enum`
at all, so row 9 is the complete out-of-range-scalar surface.
