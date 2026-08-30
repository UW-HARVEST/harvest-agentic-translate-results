# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Mechanical grep evidence

```sh
$ grep -nE 'return|assert|NULL|errno|if *\(|switch|#if|-1|ERROR|exit|abort' \
        src/driver.c include/driver.h
include/driver.h:24:#ifndef DRIVER_H_
```

The single match is the header's include guard. Therefore the C library
contains:

* **zero** `return` statements (both functions are `void`)
* **zero** `assert`s
* **zero** `NULL` / pointer validity checks
* **zero** range checks, error enums, error macros, min/max constants
* **zero** `if` / `switch` / conditional-compilation branches
* no `errno` use, no `exit`/`abort`

The library therefore has **no explicit error-return surface**. It cannot
"reject" input by value. What it *does* have is a set of **implicit rejection /
saturation / trap behaviours** that a caller can observe, and those are the
rows below. Each row is covered by a differential test in
`tests/error_paths.rs` that asserts C and Rust behave *identically* (same
printed bytes, or same fatal signal), not merely "both failed somehow".

## Table

| #  | function    | trigger (the exact invalid input/condition)                                             | expected C result |
|----|-------------|------------------------------------------------------------------------------------------|-------------------|
| 1  | `print_foo` | `foo == NULL` → unconditional `movzbl (%rax)` dereference at `driver.c:37`                | fatal `SIGSEGV` (signal 11), nothing printed |
| 2  | `print_foo` | `foo` = non-null but unmapped/dangling pointer (e.g. `0x1`)                               | fatal `SIGSEGV` (signal 11) |
| 3  | `print_foo` | `foo` = *misaligned* pointer (`buf+1`, `buf+2`, `buf+3`); no alignment check exists       | no fault on x86-64; reads byte 0 and the 4 bytes at +4 of that address, prints them |
| 4  | `driver`    | `x > 3` (out of the 2-bit field's range) — no check, silent truncation `and $0x3`         | prints `x & 3` |
| 5  | `driver`    | `x == UINT_MAX` (maximum unsigned value, worst case of row 4)                             | prints `3` for x |
| 6  | `driver`    | `y > 7` (out of the 3-bit field's range) — no check, silent truncation `and $0x7`         | prints `y & 7` |
| 7  | `driver`    | `y == UINT_MAX`                                                                           | prints `7` for y |
| 8  | `driver`    | `b` passed as a byte that is **not** a valid `_Bool` (2, 3, 0x7F, 0x80, 0xFF) — the "out-of-range enum/bool value across FFI" case; `driver` does `movzbl; and $0x1` | prints `b & 1` (e.g. `2`→`0`, `3`→`1`, `0xFF`→`1`) |
| 9  | `driver`    | `b` upper 24 bits of the argument register non-zero (e.g. `0x100`, `0xFFFFFF00`) — only `%dl` is read | prints `(b & 0xFF) & 1` → `0` |
| 10 | `driver`    | `z == INT_MIN` (`-2147483648`), one step past the positive range / extreme boundary        | prints `-2147483648` |
| 11 | `driver`    | `z == INT_MAX` (`2147483647`)                                                             | prints `2147483647` |
| 12 | `driver`    | `z == -1` (all bits set; `%d` sign handling)                                              | prints `-1` |
| 13 | `print_foo` | bit-field storage byte with bits 6..7 set (`0xC0`..`0xFF` in byte 0) — padding bits that the C code must ignore | prints only bits 0..5 decoded; padding bits invisible |
| 14 | `print_foo` | bytes 1..3 (inter-field padding) set to arbitrary garbage                                 | ignored entirely; output depends only on byte 0 and bytes 4..7 |
| 15 | `driver`    | all three "invalid" inputs at once: `x=UINT_MAX, y=UINT_MAX, b=0xFF, z=INT_MIN`            | prints `3 7 1 -2147483648` |
| 16 | `print_foo` | object placed so that **exactly 8 bytes** are readable before an unmapped page (`PROT_NONE`) | succeeds (byte 0 + dword at +4 are the only reads) |
| 17 | `print_foo` | object truncated by an unmapped page: only 1..7 bytes readable                              | fatal `SIGSEGV` for every truncation length 1..7 |

Rows 1–2 and 16–17 are *fatal* (or read-footprint-sensitive) conditions; the
tests for them run the call in a **fresh child process** and compare the child's
termination status (signal number / exit code) between C and Rust, so a
divergence such as "C segfaults / Rust panics with a Rust-specific abort", or a
Rust translation that touches *more* memory than the C does, is detected.

Rows 16–17 exist because the natural Rust translation of
`printf(..., foo->x, ..., foo->z)` is a single 8-byte struct read, whereas the C
loads byte 0 and the dword at offset 4 separately. The tests confirm the
observable read footprint is byte range `[0, 8)` for both implementations.

## Checklist

- [x] 1  `print_foo(NULL)` — same fatal signal
- [x] 2  `print_foo(0x1)` — same fatal signal
- [x] 3  `print_foo` misaligned pointers — identical output
- [x] 4  `driver` `x > 3` truncation — identical output
- [x] 5  `driver` `x == UINT_MAX` — identical output
- [x] 6  `driver` `y > 7` truncation — identical output
- [x] 7  `driver` `y == UINT_MAX` — identical output
- [x] 8  `driver` invalid `_Bool` byte — identical output
- [x] 9  `driver` wide non-zero bool register — identical output
- [x] 10 `driver` `z == INT_MIN` — identical output
- [x] 11 `driver` `z == INT_MAX` — identical output
- [x] 12 `driver` `z == -1` — identical output
- [x] 13 `print_foo` padding bits 6..7 set — identical output
- [x] 14 `print_foo` padding bytes 1..3 garbage — identical output
- [x] 15 `driver` all-invalid combination — identical output
- [x] 16 `print_foo` 8 readable bytes at a page edge — both succeed
- [x] 17 `print_foo` 1..7 readable bytes at a page edge — both `SIGSEGV`

Every row above is covered by the identically-numbered test in
`tests/error_paths.rs`, which passes in the dev and release profiles.
