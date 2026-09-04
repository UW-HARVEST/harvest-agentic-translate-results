# CONFIGS.md — configuration-surface table

Mechanically derived from the public header + every branch the C code and the
libc facilities it calls actually take.

## Axes the C code distinguishes

`c_src/include/driver.h` exports exactly one entry point:

```c
void driver(char c);
```

There is no init/teardown, no handle, no options struct, no `#ifdef`, and no
runtime flag in the public surface. The axes are therefore:

1. **Process locale state on entry** — `driver` calls `setlocale(LC_ALL, "C")`,
   so the pre-existing locale is an input that the code branches on inside
   libc (`"C"` already set / a different locale set / an invalid locale name
   previously in effect).
2. **The `char` value's class** — the twelve `<ctype.h>` predicates partition
   the 256 possible `char` values into distinct bit patterns. Every distinct
   pattern is its own shape: negative bytes, `NUL`, controls, `\t`, other
   whitespace, space, digits, hex letters, non-hex upper, non-hex lower,
   punctuation, `DEL`.
3. **Sign of the `char` parameter** — the platform `char` is signed, so
   `0x80..0xFF` reach the `is*` tables as negative indices (`-128..-1`), a
   distinct code path in glibc's table lookup from `0..127`.
4. **Value passed across FFI wider than `char`** — the ABI lets a caller pass
   any `int`; the callee truncates. Both libraries must truncate identically.
5. **stdout destination / buffering mode** — the output is produced with
   `printf`, so the byte stream is observed through a file (fully buffered), a
   pipe (fully buffered), and a tty-like fd (line buffered). Buffer flushing
   interleaving with the caller's own `printf` is part of the observable
   behaviour.
6. **Call multiplicity** — one call, repeated calls with the same value
   (idempotence / no hidden state), and many calls with different values in
   sequence (no cross-call state leakage through the static ctype tables).

## Table (one row per combination the C treats differently)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver` | locale `"C"` already set; `c` = every value in `-128..=127` (exhaustive, 256 shapes) | [x] |
| 2 | `driver` | locale `"C"`; `c` = negative half only, `-128..=-1`, randomized order | [x] |
| 3 | `driver` | locale `"C"`; `c` = `0` (`NUL`) — embedded-NUL output shape | [x] |
| 4 | `driver` | locale `"C"`; `c` = C0 controls `1..=8`, `14..=31` (cntrl, not space) | [x] |
| 5 | `driver` | locale `"C"`; `c` = `\t` (9) — cntrl + space + blank | [x] |
| 6 | `driver` | locale `"C"`; `c` = `\n \v \f \r` (10..13) — cntrl + space, not blank | [x] |
| 7 | `driver` | locale `"C"`; `c` = `' '` (32) — print + space + blank, not graph | [x] |
| 8 | `driver` | locale `"C"`; `c` = digits `'0'..'9'` — digit + xdigit + alnum | [x] |
| 9 | `driver` | locale `"C"`; `c` = `'A'..'F'` / `'a'..'f'` — alpha + xdigit + alnum, case-converting | [x] |
| 10 | `driver` | locale `"C"`; `c` = `'G'..'Z'` / `'g'..'z'` — alpha + alnum, NOT xdigit, case-converting | [x] |
| 11 | `driver` | locale `"C"`; `c` = punctuation `!"#$%&'()*+,-./:;<=>?@[\]^_`{|}~` (all 32) | [x] |
| 12 | `driver` | locale `"C"`; `c` = `127` (`DEL`) — cntrl, not print | [x] |
| 13 | `driver` | locale `en_US.UTF-8` (or any non-`"C"`) set by the caller before the call; `c` = randomized over the full range | [x] |
| 14 | `driver` | locale set to an unavailable/garbage name before the call (`setlocale` failed, locale still `"C"`); randomized `c` | [x] |
| 15 | `driver` | locale forced to `C.UTF-8`; randomized `c` — multibyte-capable locale must still be reset to `"C"` | [x] |
| 16 | `driver` | after the call, assert `setlocale(LC_ALL, NULL) == "C"` for both libraries (the side effect on global state, not just stdout) | [x] |
| 17 | `driver` | oversized/out-of-`char`-range argument passed via an `extern "C" fn(c_int)` cast: `128`, `200`, `255`, `256`, `257`, `-129`, `-200`, `-256`, `0x1FF`, `i32::MIN`, `i32::MAX` | [x] |
| 18 | `driver` | stdout redirected to a regular **file** (fully buffered); randomized `c` | [x] |
| 19 | `driver` | stdout redirected to a **pipe** (fully buffered, capacity-bounded); randomized `c` | [x] |
| 20 | `driver` | repeated calls with the SAME value (32 iterations) — output must repeat verbatim, no hidden state | [x] |
| 21 | `driver` | long randomized SEQUENCE of differing values in one capture (256 calls, fixed seed) — cross-call state leakage | [x] |
| 22 | `driver` | caller's own `printf` interleaved before/after `driver` in the same capture — buffering/interleaving parity | [x] |
| 23 | `driver` | full randomized property sweep, fixed seed, 4096 draws over `-128..=127` | [x] |
| 24 | `driver` | randomized sweep with the locale randomly perturbed between calls (axes 1 × 2 crossed) | [x] |

All 24 rows are exercised in `tests/differential.rs`.
