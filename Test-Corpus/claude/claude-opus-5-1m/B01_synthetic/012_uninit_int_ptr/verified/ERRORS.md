# ERRORS.md — error-surface table

## Mechanical derivation

Grepped `c_src/src/main.c` (60 lines, 37 of them licence header) for every
rejection construct:

| construct searched | occurrences |
|---|---|
| `RETURN_ERROR`, `return -1`, `return NULL` | **0** |
| `assert` | **0** |
| `errno`, `ERANGE`, `EINVAL` | **0** |
| explicit range check (`<`, `>`, `<=`, `>=`) | **0** |
| null check | **0** |
| `exit()` / `abort()` | **0** |
| min/max constants (`INT_MAX`, …) | **0** |
| `if` statements | **1** — `if (x)` (line 51) |
| `return` statements | **1** — `return 0` (line 59) |
| library calls | `printf`, `scanf` |

**The program contains no explicit error handling whatsoever.** `main` returns
`0` unconditionally. `scanf`'s return value is **discarded** — the sole way a
failure becomes observable is that `x` keeps its initialiser `0`, which steers
the `if (x)` branch to `bad()`. Every row below is therefore a *rejection
absorbed into the `x == 0` path*, and the correct expected result is
`stdout == "0\n"`, `exit == 0`.

The rows are the distinct ways the C reaches that state, since each drives a
different amount of `scanf` work (and thus a different libc/stack state feeding
the uninitialised-pointer read in `bad()`).

## Error-surface table

| # | function | trigger (exact invalid input/condition) | expected C result | test | status |
|---|----------|------------------------------------------|-------------------|------|--------|
| 1 | `main`/`scanf` | **Matching failure**, leading non-numeric byte (`"abc"`, `"."`, `"x"`, `"@"`) — `scanf` returns 0, `x` untouched | `stdout="0\n"`, exit 0 | `err_matching_failure_alpha` | PASS |
| 2 | `main`/`scanf` | **Input failure**, empty stdin / immediate EOF (`""`) — `scanf` returns `EOF`, `x` untouched | `stdout="0\n"`, exit 0 | `err_input_failure_empty` | PASS |
| 3 | `main`/`scanf` | **Whitespace-only** input then EOF (`"   "`, `"\n\n"`, 10 000 spaces) — skips ws, hits EOF, returns `EOF` | `stdout="0\n"`, exit 0 | `err_whitespace_only_eof` | PASS |
| 4 | `main`/`scanf` | **Sign with no digits** (`"+"`, `"-"`, `"+ "`, `"--5"`, `"- 5"`) — matching failure after sign consumed | `stdout="0\n"`, exit 0 | `err_sign_without_digits` | PASS |
| 5 | `main`/`scanf` | **stdin closed** (fd 0 closed) — read error, input failure | `stdout="0\n"`, exit 0 | `err_stdin_closed` | PASS |
| 6 | `main`/`scanf` | **Byte that is not C-locale whitespace but looks blank** (`0x85`, `0xA0`, `0x00`, `0x1C`) leading — must NOT be skipped, so matching failure | `stdout="0\n"`, exit 0 | `err_non_locale_whitespace` | PASS |
| 7 | `main`/`scanf` | **Non-decimal numeric prefixes** (`"0x10"`, `"0b1"`, `"1e5"`) — `%d` is base 10, parses `0`/stops early | value-dependent, see test | `err_non_decimal_prefix` | PASS |
| 8 | `main`/`scanf` | **Positive overflow past `LONG_MAX`** (`"9223372036854775808"`, 400 nines) — `strtol` saturates to `LONG_MAX`, `ERANGE` ignored, low 32 bits assigned to `int` | saturate-then-truncate; `"5\n"` | `err_overflow_positive` | PASS |
| 9 | `main`/`scanf` | **Negative overflow past `LONG_MIN`** (`"-9223372036854775809"`, `-1`×400 nines) — saturates to `LONG_MIN` = `0x8000…0`, low 32 bits = `0` | `stdout="0\n"`, exit 0 | `err_overflow_negative` | PASS |
| 10 | `main`/`scanf` | **In-range value whose low 32 bits are 0** (`"4294967296"`, `m·2^32`) — no error, but truncation flips the branch to `bad()` | `stdout="0\n"`, exit 0 | `err_truncation_to_zero` | PASS |
| 11 | `printIntPtrLine`/`bad` | **Uninitialised pointer dereference** (UB) — reached whenever `x == 0` | reference `-O0` build reads a zeroed libc slot; `stdout="0\n"`, exit 0, no crash | `err_uninitialised_ptr_deref_stable` | PASS |
| 12a | `printf` | **stdout fd closed** (`close(1)`) — write fails `EBADF`; `printf`'s return value is ignored by the C code | exit 0, no output | `err_stdout_closed` | PASS |
| 12b | `printf` | **stdout is a readerless pipe** — write raises `SIGPIPE`, left at `SIG_DFL` by C | **killed by signal 13** (status 141), no output | `err_stdout_epipe`, `cfg_stdout_epipe_sigpipe_parity` | PASS (bug found & fixed, see below) |

## Generic FFI boundaries (required even though absent from the table)

| boundary | applicability here | covered by |
|---|---|---|
| Null pointers across FFI | **n/a** — no exported functions, no pointer parameters cross any boundary (`nm -D` = 0 symbols) | — |
| Out-of-range enum values across FFI | **n/a** — the program declares no enums and no FFI entry points | — |
| Zero / oversized lengths | maps to zero-length stdin and oversized digit runs | rows 2, 3, 8, 9 + `prop_long_digit_runs` (up to 20 000 bytes) |
| One step past a valid range | `INT_MAX±1`, `LONG_MAX±1`, `2^32±1`, `2^64±1` | rows 8, 9, 10 + `boundary_one_past_range` |

Rows 1–12: **12/12 checked and passing.**

---

## Divergence found and FIXED during Phase C

**One real translation bug was found**, on the ERRORS row 12 path.

| | |
|---|---|
| **Trigger** | `stdout` is a pipe with no reader (e.g. `driver \| true`, or any consumer that exits before reading) |
| **C reference** | `printf` write raises `SIGPIPE`; disposition is `SIG_DFL`, so the process is **killed by signal 13** (shell wait status **141**) |
| **Rust (before fix)** | Rust's runtime installs `SIG_IGN` for `SIGPIPE` before `main` runs, so the write returned `EPIPE`, the ignored-error `let _ = writeln!` swallowed it, and the process **exited 0** |
| **Impact** | Wrong exit status and wrong termination mode on every piped invocation — 100 % reproducible, on both branches (`x == 0` and `x != 0`) and both cargo profiles |
| **Fix** | `restore_default_sigpipe()` in `src/main.rs` resets `SIGPIPE` to `SIG_DFL` at the top of `main`, before any write |
| **Regression test** | `err_stdout_epipe`, `cfg_stdout_epipe_sigpipe_parity` |

Measured before the fix:

```
C    -> code=None    signal=Some(13)  stdout=""
Rust -> code=Some(0) signal=None      stdout=""
```

and after:

```
C    -> code=None signal=Some(13) stdout=""
Rust -> code=None signal=Some(13) stdout=""
```

The regression test was **mutation-checked**: with `restore_default_sigpipe()`
commented out, `err_stdout_epipe` fails with exactly the diff above, confirming
the test is not vacuous.

### Note on harness determinism

`run_with_readerless_stdout` closes the pipe's read end *before* spawning the
child. Closing it after `spawn` is racy under parallel test execution: an
unrelated test's child, forked in the window before its own `exec`, transiently
holds an inherited copy of the read end and suppresses the signal, which made
the test flaky (passing alone, failing under load). Closing first removes the
race entirely — the write end has zero readers from the outset.
