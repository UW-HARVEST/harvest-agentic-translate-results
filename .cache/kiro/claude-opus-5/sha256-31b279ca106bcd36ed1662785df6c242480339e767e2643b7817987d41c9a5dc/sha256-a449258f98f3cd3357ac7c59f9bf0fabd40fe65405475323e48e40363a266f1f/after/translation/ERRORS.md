# Differential verification log

Reference: `c_src/src/main.c` (built with CMake, no optimization flags — the
default empty `CMAKE_BUILD_TYPE`).
Subject: `translation/src/main.rs` (`cargo build --release`).

Both programs are compared by execution only: the built executables are spawned
as subprocesses with identical stdin, and **stdout, stderr and exit status**
(including death-by-signal) are compared byte for byte. See
`translation/tests/differential.rs`.

Run commands:

| program | command |
|---|---|
| C | `c_src/build/driver` |
| Rust | `translation/target/release/driver` |

---

## Mismatches found and fixed

### 1. `SIGPIPE` was ignored, so the Rust program exited 0 where the C died

**Symptom.** With stdout connected to a pipe whose read end had been closed, the
two programs disagreed on exit status for any input that produces output:

```
input "50\n", stdout = pipe with no reader
  C    : killed by signal 13 (SIGPIPE)
  Rust : exit code 0
```

stdout and stderr were both empty in each case, so a test that only compared
those two streams would have passed. Only the exit-status comparison caught it.

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before
`main` runs. A write to a broken pipe therefore returns `EPIPE` instead of
killing the process, and because the translation ignores write errors (as C's
unchecked `printf` does), the program ran to completion and returned 0. The C
program keeps the default disposition and is killed by the signal.

**Fix.** `restore_default_sigpipe()` in `translation/src/main.rs` calls
`signal(SIGPIPE, SIG_DFL)` as the first statement of `main`, restoring the
disposition a C program starts with. Covered by the `broken_pipe_kills_both`
test.

---

## Behaviors that were already correct, and are load-bearing

These are not mismatches — they were verified, not fixed. They are recorded
because each one is a place where a straightforward translation would diverge,
so a future edit that "cleans them up" would reintroduce a bug.

### `strncpy` with a negative length must fault

`data` is an `int` and reaches `strncpy(dest, source, data)` unchecked. For any
negative `data` the length is sign-extended into a `size_t` near `SIZE_MAX`, so
`strncpy` copies `source` and then pads far past the end of the 100-byte `dest`,
and the process dies from `SIGSEGV`. There is no negative value for which the
length is small, so *every* negative `data` faults.

The translation reproduces this with a deliberate wild store
(`out_of_bounds_write_fault`) rather than by returning an error. Verified
identical (signal 11, empty stdout, empty stderr) for `-1`, `-5`, `-99`,
`-100`, `-2147483648`, and via `atoi` wraparound.

### The `fgets` failure message is *lost* when stdout is a pipe

On empty stdin, `fgets` returns `NULL`, `printLine("fgets() failed.")` runs, and
then `data` is still `-1` so the program faults. In C, stdout attached to a pipe
is fully buffered, so the message is still in the buffer when the process dies
and never reaches the pipe. The observable stdout is **empty**, not
`"fgets() failed.\n"`.

The translation models this with `CStdout`: it buffers, only flushes eagerly
when `stdout` is a terminal (`is_terminal()`), and does not flush on the fault
path. Confirmed both ways — under a pipe both programs emit nothing, and under a
pty (via `script`) both emit `fgets() failed.\r\n` before dying.

### `atoi` truncates a `long` to `int`, which can turn a large input negative

glibc's `atoi` is `(int) strtol(s, NULL, 10)`. The 13-byte input window means up
to 13 digits reach it, which fits a 64-bit `long`, and the result is then
truncated to 32 bits. So:

| input | `data` | result |
|---|---|---|
| `2147483648` | `-2147483648` | faults |
| `4294967296` | `0` | prints an empty line |
| `4294967396` | `100` | guard fails, prints an empty line |
| `9999999999999` | `1215752191`-style wrap | guard fails |

On digit overflow past `LONG_MAX`, `strtol` saturates *before* the truncation,
so `99999999999999999999999999` yields `LONG_MAX` and truncates to `-1`, which
faults. The translation reproduces the saturate-then-truncate order.

### `fgets(inputBuffer, 14, stdin)` reads at most 13 bytes and stops at `\n`

It does not skip leading whitespace and does not read across the newline, so
only the first line — truncated to 13 bytes — is ever parsed. `1234567890123456`
is read as `1234567890123`, which changes the value. Trailing lines are ignored
entirely.

### `if (data < 100)` leaves `dest` untouched, not unwritten

When `data >= 100` the copy is skipped and `dest`, initialized to `""`, is
printed as an empty string — so the program still emits a single `\n`. The
guard is `< 100`, so `99` is the largest length copied (99 `A`s).

### Embedded NUL bytes

`fgets` copies NUL bytes into the buffer verbatim; `atoi` then treats the first
NUL as end-of-string. `"\x005\n"` parses as `0`, not `5`. The translation's
`atoi` stops at any non-digit, which covers NUL without special-casing it.

---

## Coverage

Every branch in `main` is reached by the suite:

| branch | test |
|---|---|
| `fgets(...) != NULL` true | most tests |
| `fgets(...) != NULL` false (`printLine("fgets() failed.")`) | `fgets_failure_path_empty_stdin`, `stdin_immediately_at_eof` |
| `data < 100` true, `data == 0` | `zero_length_copy` |
| `data < 100` true, `0 < data <= 99` | `every_in_range_length` (all of 0..=99), `single_item` |
| `data < 100` true, `data < 0` (faults) | `negative_length_faults`, `atoi_truncation_to_int` |
| `data < 100` false | `upper_boundary_of_copy` |
| `printLine` with non-NULL | every test |
| `printLine` with NULL | unreachable — neither call site can pass NULL |

Plus: `atoi_prefix_parsing`, `fgets_stops_at_newline`,
`fgets_thirteen_byte_window`, `input_without_trailing_newline`,
`embedded_nul_bytes`, `stdout_closed`, `broken_pipe_kills_both`,
`numeric_sweep` (boundaries of the guard, `INT_MAX`, the sign boundary) and
`randomized_sweep` (150 fixed-seed byte-soup inputs).

An additional external fuzz run of 4,488 inputs over the alphabet
`0-9 space tab newline + - a b c X Y NUL CR VT FF 0xff . e E` produced zero
mismatches in stdout, stderr or exit status.

`printLine`'s NULL check is dead code in this program; it is retained in the
translation as an `Option<&[u8]>` parameter so the structure still mirrors the C.
