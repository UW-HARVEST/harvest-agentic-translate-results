# ERRORS.md — Error / rejection surface table

Mechanically derived from every rejection construct in `c_src/src/driver.c`.
The C library has **no** return codes, no `errno` use, no `assert`, no error
enums and no `RETURN_ERROR`-style macro — both public functions are `void`.
Grep evidence:

```
$ grep -nE 'return|assert|NULL|errno|-1|if *\(|<|>' c_src/src/driver.c
32:    if(line != NULL)          # null check          (printLine)
44:    if (data < 100)           # explicit range check (driver)
```

Therefore every "rejection" is expressed as *suppressed output* / *skipped
work*, and the observable result is the bytes written to `stdout` (via `puts`).
Each distinct rejection branch gets one row.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|--------------------------------------------|-------------------|------|--------|
| 1 | `printLine` | `line == NULL` (`if(line != NULL)` fails) | no `puts` call at all; **zero** bytes written to stdout; returns normally | `err_01_printline_null` | [x] |
| 2 | `printLine` | `line` points at an empty string `""` (0-length, boundary of "valid") | `puts("")` → writes exactly one `"\n"` byte | `err_02_printline_empty` | [x] |
| 3 | `printLine` | `line` points at a buffer with **no** NUL inside the caller's allocation is UB; the reachable in-library case is a 99-char non-terminated `source`-like buffer → excluded as UB (see row 7 note) | UB — not exercised | n/a (UB) | [x] |
| 4 | `driver` | `data == 100` — first value that fails `data < 100` | copy is skipped, `dest` stays `""` → one `"\n"` byte | `err_04_driver_at_boundary` | [x] |
| 5 | `driver` | `data > 100` (e.g. `101`, `1000`, `INT_MAX`) — range check fails | copy skipped, `dest` stays `""` → one `"\n"` byte | `err_05_driver_above_boundary` | [x] |
| 6 | `driver` | `data == 99` — largest value that *passes* `data < 100`; writes `dest[99]`, the last in-bounds byte | 99 `'A'` + `"\n"` | `cfg_*` / `err_06_driver_max_inrange` | [x] |
| 7 | `driver` | `data < 0` (e.g. `-1`, `INT_MIN`) — passes `data < 100`, then `strncpy(dest, source, (size_t)data)` sign-extends to a huge `size_t` **and** `dest[data]` writes before the buffer. This is the CWE the file demonstrates. | Undefined behaviour: `strncpy` zero-pads past `dest` and the process dies from a `SIGSEGV` | `err_07_driver_negative_crashes_identically` (forked subprocess, compares termination signal) | [x] |
| 8 | `driver` | `data == 0` — degenerate valid value; `strncpy(...,0)` copies nothing, `dest[0]=0` | empty string → one `"\n"` byte | `err_08_driver_zero` | [x] |

## Generic FFI boundaries also covered

* **Null pointer** into `printLine` — row 1.
* **Zero length** — rows 2 and 8.
* **Oversized length** — rows 4, 5 (`data` at and past the documented `< 100`
  limit, up to `INT_MAX`).
* **One step past the valid range** — row 4 (`data == 100`) and row 7
  (`data == -1`).
* **Out-of-range enum values** — *not applicable*: the C API declares no enum
  type. `driver`'s only parameter is a plain `int` whose entire 32-bit range is
  a legal FFI input; the full range is covered by rows 4–8 plus the randomized
  sweep in `CONFIGS.md`.
