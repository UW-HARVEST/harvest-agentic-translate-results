# ERRORS.md — error-surface table

Mechanically derived from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Mechanical grep of every rejection construct in the C source

```
$ grep -n 'return\|assert\|RETURN_ERROR\|NULL\|errno\|exit\|abort\|if (\|if(\|<\|>' c_src/src/driver.c
```

Result: `driver()` contains **no** `return` statement, **no** `assert`, **no**
`if`, **no** range check, **no** null check, **no** error enum, and **no**
`errno` use. It is `void`-returning and unconditionally executes 15 library
calls. `setlocale`'s return value (which can be `NULL`) is **discarded**.

Therefore the library has no *explicit* error surface. The table below is the
complete set of *implicit* rejection/boundary conditions — the places where the
underlying libc contract has a limit that the C code silently relies on. Each
row is a real input the C accepts, and the Rust must reproduce its result
exactly rather than reject it.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `driver` | `setlocale(LC_ALL, "C")` returns `NULL` (locale unavailable) — return value never checked | ignored; execution continues and all 14 `printf`s still run |
| 2 | `driver` | `c == 0` (`'\0'`, the NUL char — a valid `char`, not a string terminator) | all 12 predicates print `0`; `to lower:`/`to upper:` print a literal NUL byte |
| 3 | `driver` | `c == -1` (`0xFF`) — negative index into glibc's `__ctype_b` table | all 12 predicates `0`; `tolower`/`toupper` return `-1`, `%c` narrows to byte `0xFF` |
| 4 | `driver` | `c == -128` (`0x80`) — the most negative `char`, lowest legal table index | all 12 predicates `0`; `%c` emits byte `0x80` |
| 5 | `driver` | `c == 127` (`DEL`) — highest positive `char`, boundary of the ASCII table | `control: 2`, everything else `0`; `%c` emits byte `0x7F` |
| 6 | `driver` | `c` in `-128 ..= -1` generally (any high-bit-set byte) — the whole negative half of the index range | every predicate `0`; case conversion is the identity, so `%c` reproduces the original byte |
| 7 | `driver` | an out-of-`char`-range `int` passed across FFI (e.g. `200`, `256`, `-200`, `0x1FF`) — C enums/`char` params accept any int at the ABI level | the value is truncated to 8 bits by the `char` parameter, so it aliases an in-range `char`; no rejection, no UB signalled |
| 8 | `driver` | `c == 32` (`' '`) — the one value that is `isprint` **and** `isspace` **and** `isblank` but **not** `isgraph`/`ispunct` | `space: 8192`, `blank: 1`, `printing: 16384`, all others `0` |
| 9 | `driver` | `c == 9` (`'\t'`) — the one value that is `iscntrl` **and** `isblank` **and** `isspace` | `control: 2`, `space: 8192`, `blank: 1`, others `0` |
| 10 | `driver` | `c == 31` / `c == 33` / `c == 126` — one step past each documented `isprint`/`isgraph` range boundary | `31`→`control: 2` only; `33`,`126`→`graphical: 32768`,`printing: 16384`,`punctuation: 4` |
| 11 | `driver` | `c == '/'` (47) and `c == ':'` (58) — one step below/above the `isdigit` range | punctuation, `digit: 0`, `hexadecimal: 0` |
| 12 | `driver` | `c == '@'` (64) / `c == '['` (91) / `` c == '`' `` (96) / `c == '{'` (123) — one step outside each alpha range | punctuation only; `alphabetic: 0`, case conversion is identity |
| 13 | `driver` | `c == 'G'` (71) and `c == 'g'` (103) — one step past the `isxdigit` letter ranges | `hexadecimal: 0` while `alphabetic: 1024` |
| 14 | `driver` | stdout is a closed/failed fd, so every `printf` returns `< 0` | return value discarded; `driver` still returns normally |
| 15 | `driver` | caller's locale is *not* `"C"` on entry (e.g. `en_US.UTF-8`) | `setlocale(LC_ALL, "C")` overwrites it; classification uses the `"C"` tables, and the process locale is left as `"C"` after the call |

## Status

All 15 rows are covered by `tests/differential.rs` (see the `errors_row_*`
tests and `exhaustive_all_char_values`, which subsumes rows 2–13 by covering
every one of the 256 possible `char` values).

- [x] 1 · [x] 2 · [x] 3 · [x] 4 · [x] 5 · [x] 6 · [x] 7 · [x] 8
- [x] 9 · [x] 10 · [x] 11 · [x] 12 · [x] 13 · [x] 14 · [x] 15
