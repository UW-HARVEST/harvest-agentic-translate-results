# ERRORS.md — Phase C error-surface table

## How this was derived

`c_src/src/main.c` (155 lines) contains **no** `assert`, no error enum, no `return -1`,
no `return NULL`, and no error-return macro. Every way the program rejects or reacts to
bad input is one of:

* a guarded `if (...) { ... } else { printLine("...") }` pair, or
* the `line != NULL` guard in `printLine`, or
* the **missing** upper-bound check in `bad()` (the CWE-129 defect itself).

Enumerated mechanically:

```
grep -nE 'if\s*\(|else|return|NULL|assert|printLine\(' c_src/src/main.c
```

which yields exactly 6 conditionals (lines 29, 47, 60, 87, 111, 124), and the only
numeric bounds in the file are `14` (the `fgets` size, line 46/110) and `10` (the array
length and `goodB2G`'s upper bound, lines 59/124).

`atoi()` is worth calling out because it is **not** an error path: glibc `atoi` reports
no failure at all. An unparseable string yields `0`, which is a perfectly valid index, so
`"abc"` is not rejected — it behaves exactly like `"0"`. This is a real trap and is
covered by rows 10–11.

## The table

Sink notation: `data` is the value fed to the array index. `stdin` supplies two lines —
the first is consumed by `goodB2G`, the second by `bad()`.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test |
|----|----------|----------------------------------------------|-------------------|------|
| 1  | `printLine` (line 29) | `line == NULL` | guard fails; prints **nothing**, no newline, returns normally | `err_printline_null` |
| 2  | `printLine` (line 29) | `line` = valid pointer to empty string `""` | guard passes; prints just `"\n"` | `err_printline_empty` |
| 3  | `printLine` (line 29) | `line` = non-UTF-8 bytes (e.g. `\xff\xfe`) + NUL | bytes emitted verbatim + `"\n"` | `err_printline_non_utf8` |
| 4  | `bad` (line 47) | stdin at EOF before any byte → `fgets` returns NULL | prints `"fgets() failed."`; `data` stays `-1` → falls into row 5 | `err_bad_fgets_eof` |
| 5  | `bad` (line 60) | `data < 0` (e.g. input `"-1"`, or EOF via row 4) | prints `"ERROR: Array index is negative."`; no array values printed | `err_bad_negative` |
| 6  | `bad` (line 62) | `data >= 10` — **no upper-bound check exists** (CWE-129) | out-of-bounds 4-byte store at `&buffer[0]+4*data`; consequences are fully determined by the gcc `-O0` frame layout — see `CONFIGS.md` rows 12–17 and the frame-layout table in `src/imp.rs` | `err_bad_oob_*` |
| 7  | `goodB2G` (line 111) | stdin at EOF before any byte → `fgets` returns NULL | prints `"fgets() failed."`; `data` stays `-1` → falls into row 8 | `err_b2g_fgets_eof` |
| 8  | `goodB2G` (line 124) | `data < 0` (first conjunct fails) | prints `"ERROR: Array index is out-of-bounds"` | `err_b2g_negative` |
| 9  | `goodB2G` (line 124) | `data >= 10` (second conjunct fails) — **guarded**, so no OOB write | prints `"ERROR: Array index is out-of-bounds"` | `err_b2g_too_large` |
| 10 | `goodB2G` / `bad` (line 114/50) | unparseable text (`"abc"`, `"x"`, `"+"`, `"-"`, `".5"`, `"0x10"`) | `atoi` returns `0` — **not** an error; index `0` is used and `buffer[0]=1` is printed | `err_atoi_unparseable` |
| 11 | `goodB2G` / `bad` (line 114/50) | empty line `"\n"` only | `fgets` succeeds (returns `"\n"`, non-NULL), `atoi("\n")` → `0`; index `0` used | `err_atoi_empty_line` |
| 12 | `goodB2G` / `bad` (line 114/50) | value above `INT_MAX` but ≤ 13 digits (`"9999999999999"`) | `strtol` → `9999999999999`, `(int)` truncates low 32 bits → `1316134911` → row 9 (`goodB2G`) / row 6 (`bad`) | `err_atoi_int_truncation` |
| 13 | `goodB2G` / `bad` (line 114/50) | `"-9999999999999"` (14 chars, so `fgets` keeps only 13: `"-999999999999"`) | `strtol` → `-999999999999`; `(int)` truncation makes it **positive**: `727379969`. So it is rejected by `goodB2G` as *out-of-bounds*, **not** as negative — sign is not preserved by the truncation | `err_atoi_neg_truncation` |
| 14 | `goodB2G` / `bad` (line 114/50) | exactly `"-2147483648"` / `"2147483647"` (INT_MIN / INT_MAX) | returned unchanged; INT_MIN → negative rows 5/8; INT_MAX → row 9 / row 6 | `err_atoi_int_limits` |
| 15 | `goodB2G` / `bad` (line 114/50) | embedded NUL, e.g. `"5\x006\n"` | `fgets` stores all bytes, but `atoi` stops at the NUL → `5` | `err_atoi_embedded_nul` |
| 16 | `bad` / `goodB2G` (line 47/111) | line **longer than 13 bytes** | `fgets(.,14,.)` truncates to 13 bytes; the remainder stays in `stdin` and is consumed by the **next** `fgets`, so one long line feeds *both* sinks | `err_fgets_truncation` |
| 17 | `bad` / `goodB2G` (line 47/111) | final line with **no trailing newline** | `fgets` succeeds and returns the bytes without a `\n`; not an error | `err_fgets_no_newline` |
| 18 | `bad` (line 47) | only **one** line of input (second `fgets` hits EOF) | `goodB2G` consumes it; `bad`'s `fgets` returns NULL → row 4 then row 5 | `err_bad_second_fgets_eof` |
| 19 | `goodG2B` (line 87) | `data < 0` — **unreachable**: line 83 hardcodes `data = 7` | `else` branch is dead code; `goodG2B` always prints `0 0 0 0 0 0 0 1 0 0` regardless of input | `err_g2b_else_unreachable` |
| 20 | `main` (line 154) | any input whatsoever, when no OOB crash occurs | returns `0` → exit status `0` | asserted by every differential test |

## Generic FFI boundary cases (required even though not in the C's own checks)

| #  | entry point | trigger | expected C result | test |
|----|-------------|---------|-------------------|------|
| 21 | `printLine` | NULL pointer (row 1 restated as the generic null-pointer case) | no output | `err_printline_null` |
| 22 | `printIntLine` | `INT_MIN` (`-2147483648`) | prints `-2147483648\n` (no overflow in `%d`) | `err_printintline_extremes` |
| 23 | `printIntLine` | `INT_MAX`, `0`, `-1` | prints the decimal form verbatim | `err_printintline_extremes` |
| 24 | `printIntLine` | out-of-`int` bit pattern passed as `c_int` (e.g. `0x80000000` reinterpreted) | reinterpreted as the negative `int`; `%d` prints it | `err_printintline_extremes` |
| 25 | `main` | `argc`/`argv` ignored by the body: `argc=0`, `argv=NULL` | no crash, identical output, returns `0` | `err_main_null_argv` |
| 26 | `bad` / `good` | called with fd 0 closed (read fails, not just EOF) | `fgets` returns NULL → rows 4/7 | `err_stdin_closed` |
| 27 | whole program | stdout is a pipe whose **read end is closed**, so `printf` gets `EPIPE` | C runs with `SIGPIPE` at its default disposition and is **killed by signal 13**. (Rust's runtime sets `SIGPIPE` to `SIG_IGN` before `main`, which would silently swallow the write and exit 0; `src/main.rs` restores `SIG_DFL`.) | `sigpipe_kills_both_programs` |
| 28 | whole program | the out-of-bounds store faults **before** the ten values are printed (far index) | process dies; on a pipe stdout is empty, on a tty exactly the 121 bytes written so far survive | `tty_line_buffering_and_crash_timing_match` |
| 29 | whole program | the store poisons `bad()`'s **own return address** (index 18/19) | the ten values are printed, then the fault at `bad`'s `ret` — `"Finished bad()"` is never emitted (151 bytes on a tty) | `tty_line_buffering_and_crash_timing_match` |
| 30 | whole program | the store poisons a frame at or above `main` (index 16/17/26/27) | everything is printed, then the fault as `main` returns (167 bytes on a tty) | `tty_line_buffering_and_crash_timing_match` |

## Enum arguments

The C API has **no enum parameters** and no `switch` statement, so the
"out-of-range enum value across the FFI boundary" class does not apply here. The nearest
analogue — an arbitrary `int` reaching a value-dispatched code path — is the array index
in `bad()`, and it is covered exhaustively for `data ∈ [-8, 200]` plus randomized 32-bit
values by rows 5, 6, 9 and `CONFIGS.md` rows 12–17.

> Rows 28–30 are the *timing* of the rejection rather than its occurrence. They are only
> observable when `stdout` is line-buffered (a terminal); on a pipe all three collapse to
> "no output at all". Both cases are asserted, and the contrast between them is what
> pins down C's buffering-mode switch.

## Status

All 30 rows have a passing differential test:

| rows | test file |
|------|-----------|
| 1–20, 22–26 | `tests/error_paths.rs` (25 tests) |
| 1–3, 21–26 | additionally through the `.so` FFI boundary in `tests/ffi_diff.rs` |
| 6 (index classes) | `tests/exe_diff.rs` rows 12–17 |
| 27–30 | `tests/stdio_semantics.rs` |

See `RESULTS.md` for the run log.
