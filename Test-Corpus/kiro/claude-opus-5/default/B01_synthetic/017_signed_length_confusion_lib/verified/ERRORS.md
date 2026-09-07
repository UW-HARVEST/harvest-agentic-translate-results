# ERRORS.md — error / rejection surface table (Phase C)

Derived mechanically from `c_src/src/driver.c` and `c_src/include/driver.h`, not
from documentation or assumptions.

## Mechanical inventory of the C source

Exhaustive grep of every construct that could reject, error, or bound an input
(`if`, `else`, `switch`, `case`, `while`, `for`, `return`, `assert`, `NULL`,
`ERROR`, `errno`, `exit(`, `abort`, `goto`, array bounds, numeric constants)
across both C files yields exactly these non-comment hits:

```
src/driver.c:32:    if(line != NULL)          <- rejection #1 (null check)
src/driver.c:40:    char source[100];         <- bound constant 100
src/driver.c:41:    char dest[100] = "";      <- bound constant 100
src/driver.c:42:    memset(source, 'A', 100-1);
src/driver.c:43:    source[100-1] = '\0';
src/driver.c:44:    if (data < 100)           <- rejection #2 (range check)
include/driver.h:24-29:  DRIVER_H_ include guard only
```

Consequences of that inventory, stated explicitly because they shape the table:

* There are **exactly two** conditional branches in the whole library, so there
  are exactly two branches that reject input.
* There are **no** error-return macros (`RETURN_ERROR` &c.), **no** `return -1`
  / `return NULL`, **no** error enums, **no** `assert`, **no** `errno` use, and
  **no** `exit`/`abort`. Both public functions are declared `void` and return
  nothing.
* Therefore a rejection is **never** signalled to the caller by a return value.
  The only observable effects are **stdout bytes** and **process termination
  status**. Every "expected C result" column below is expressed in exactly those
  terms, and every differential test asserts on exactly those terms.
* There are **no enums** anywhere in the public API, so the "out-of-range enum
  value across the FFI boundary" class collapses onto `int data`, whose full
  range `[INT_MIN, INT_MAX]` is a valid C argument and is covered by rows 5-11.
* The only unchecked argument is `data`. The C validates it **only** against the
  upper bound (`data < 100`); it never validates the lower bound. Negative
  values therefore pass the guard and reach `strncpy`'s `size_t n` parameter and
  the `dest[data]` store. Rows 8-11 record that the C's real behaviour there is
  a fatal `SIGSEGV`, and require the Rust to fail identically. Per the task
  rules this defect is reproduced, not fixed.

`source` is always 99 `'A'` bytes then `'\0'`, so for any `0 <= data <= 99`
`strncpy` copies exactly `data` `'A'` bytes and `dest[data] = '\0'` terminates
them; `driver` prints `data` `'A'`s then `'\n'`.

## Error-surface table

Expected C results below are the measured ground truth of
`c_src/build/libdriver.so` (captured through a `dlopen` probe), not predictions.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `printLine` | `line == NULL` — fails the `if(line != NULL)` check at `driver.c:32` | rejected silently: `printf` is never reached, **0 bytes** on stdout, returns normally (no crash) | `err_01_printline_null` | [x] |
| 2 | `driver` | `data == 100` — first value that fails `if (data < 100)` at `driver.c:44`; one step past the accepted range | copy block skipped; `dest` still the zero-filled `""`; prints `0a` (**1 byte**, a lone newline), returns normally | `err_02_driver_at_upper_bound` | [x] |
| 3 | `driver` | `data == 101` — one further past the bound | same as #2: `0a`, 1 byte | `err_03_driver_past_upper_bound` | [x] |
| 4 | `driver` | `data == INT_MAX` (`2147483647`) — maximal oversized length | same as #2: `0a`, 1 byte (guard rejects before any copy, so no overflow occurs) | `err_04_driver_int_max` | [x] |
| 5 | `driver` | `data == 99` — last value that *passes* the guard (one step inside the range); also the exact `strlen(source)` boundary | accepted: 99 `'A'` + `0a` = **100 bytes** | `err_05_driver_last_accepted` | [x] |
| 6 | `driver` | `data == 0` — zero length; `strncpy(dest, source, 0)` copies nothing | accepted: `dest` stays `""`, prints `0a` (**1 byte**) — indistinguishable from the rejected rows #2-#4 | `err_06_driver_zero_length` | [x] |
| 7 | `driver` | every value in the whole accepted band `data ∈ [0, 99]` (all 100 values, exhaustively) | accepted: `data` `'A'` bytes then `0a` = `data + 1` bytes | `err_07_driver_accepted_band_exhaustive` | [x] |
| 8 | `driver` | `data == -1` — **unchecked lower bound**: passes `data < 100`, then `(size_t)(-1)` = `0xFFFFFFFFFFFFFFFF` is `strncpy`'s length and `dest[-1]` writes below the buffer | fatal: terminated by **`SIGSEGV`** (wait status 139 / `128+11`), **0 bytes** flushed to stdout | `err_08_driver_negative_one` | [x] |
| 9 | `driver` | `data == -2` | fatal `SIGSEGV`, 0 bytes stdout | `err_09_driver_negative_two` | [x] |
| 10 | `driver` | `data == -100` (magnitude past the buffer size) | fatal `SIGSEGV`, 0 bytes stdout | `err_10_driver_negative_hundred` | [x] |
| 11 | `driver` | `data == INT_MIN` (`-2147483648`) — extreme negative | fatal `SIGSEGV`, 0 bytes stdout | `err_11_driver_int_min` | [x] |

Rows 8-11 cannot be asserted in-process (they abort the test process), so their
differential tests re-exec a subprocess helper that `dlopen`s one library, calls
`driver`, and exits; the test compares the **termination signal** and the
**stdout bytes** of the C run against the Rust run. That is a comparison of the
same fatal sentinel, not merely "both failed somehow".

## Generic FFI boundary classes required by Phase C

| class | covered by | note |
|-------|-----------|------|
| null pointer | row 1 | the only pointer parameter in the API is `printLine`'s `line` |
| zero length | row 6 | `data == 0` |
| oversized length | rows 2, 3, 4 | `data >= 100`, up to `INT_MAX` |
| one step past a valid range | rows 2 (=100, first rejected) and 5 (=99, last accepted) | both sides of the only bound in the library |
| out-of-range enum value | rows 5-11 | no enum exists in this API; the equivalent is `int data` outside `[0,99]`, covered on both the positive and the negative side |
| negative where unsigned is expected | rows 8-11 | `int` → `size_t` conversion at `strncpy`'s `n`; the C's real (crashing) behaviour is reproduced, not fixed |

## Verification result

All 11 rows pass. Every row has a named differential test in
`translation/tests/differential.rs`; each test drives the C `.so` and the Rust
`.so` through `dlopen`/`dlsym` and compares the **exact** rejection sentinel:

* rows 1-7 compare the stdout bytes **and** the normal-termination status, and
  additionally assert the exact C ground-truth bytes, so a mutually-wrong result
  cannot pass;
* rows 8-11 compare the **exact fatal signal** (`SIGSEGV`, wait status 139) and
  the stdout bytes, and separately assert that the C side really is killed by
  `SIGSEGV` — so "both failed somehow" is not accepted;
* `err_12_driver_negative_sweep` adds 12 further negative values (fixed plus
  seeded-random, including `INT_MIN + 1`) to show the fatal outcome is uniform
  across the entire unchecked lower range, not just the four tabulated values.

The suite passes under the default feature set and `--no-default-features`,
against both the debug and the release Rust `.so`, run both in parallel and
single-threaded — 8 configurations, 31 tests each.

### Anti-vacuity evidence (mutation testing)

A green suite only means something if it can go red. Six deliberate,
behaviour-changing mutations were injected into `translation/src/lib.rs` one at a
time; **all six were caught**, and `src/lib.rs` was then restored byte-for-byte
(verified with `cmp`):

| mutation | detected |
|----------|----------|
| guard `data < 100` → `data <= 100` (off-by-one on the only bound) | yes — 3 tests failed |
| fill byte `'A'` → `'B'` | yes — 10 tests failed |
| `printLine` NULL check removed | yes — 2 tests failed |
| `dest` no longer zero-initialised | yes |
| `"%s\n"` → `"%s"` (trailing newline dropped) | yes |
| negative `data` "fixed" by clamping to 0 — i.e. repairing the C's defect | yes — 5 tests failed |

The last row is the important one: it confirms the suite enforces
*bug-for-bug* fidelity to the C and would reject a Rust translation that
silently made the library memory-safe.

One further mutation, `memset(source,'A',100-1)` → `memset(source,'A',100)`, was
**not** detected — correctly so: it is a semantically **equivalent** mutant,
because the very next statement `source[99] = '\0'` overwrites the only byte the
change affects. It is not a coverage gap.
