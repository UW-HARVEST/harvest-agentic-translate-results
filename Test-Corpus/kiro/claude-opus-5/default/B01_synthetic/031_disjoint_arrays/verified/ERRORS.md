# Differential verification log

C reference: `c_src/src/main.c`, built with CMake to `c_src/build/driver`.
Rust translation: `translation/src/main.rs`, built to
`translation/target/{debug,release}/driver`.

Both programs are compared by execution only: same bytes on stdin, then stdout,
stderr and exit status are diffed. Tests live in `translation/tests/differential.rs`.

## What the C actually does

`main` calls `scanf("%d", &data[i])` up to 100 times, breaking out of the loop
the first time the return value is not 1 (matching failure *or* EOF). It then
calls `call_fma(data, i)`, which returns 0 when `i == 0` and otherwise computes
`out[k] = 1 * data[k] + 0` for every `k` and returns `out[i-1]`. So the program
prints the last successfully parsed integer, or `0` if none was parsed, followed
by a newline, and always returns 0.

## Mismatches found

### 1. SIGPIPE disposition — exit status differed

* **Symptom**: with stdout connected to a pipe whose reader had already closed,
  the C program was killed by `SIGPIPE` (wait status reported signal 13, i.e.
  shell exit code 141), while the Rust program completed and exited 0. stdout
  and stderr were both empty in each case, so a stdout-only comparison would
  never have caught it.
* **Cause**: the Rust standard library sets `SIGPIPE` to `SIG_IGN` before
  `main` runs. `write` to the dead pipe therefore returned `EPIPE`, which the
  translation discarded (`let _ = write!(...)`), and the process exited
  normally. The C program keeps the default disposition, so the same `write`
  raised a fatal signal.
* **Fix**: `restore_default_sigpipe()` in `translation/src/main.rs` calls
  `signal(SIGPIPE, SIG_DFL)` as the first statement of `main`, restoring the C
  disposition. Covered by
  `stdout_reader_gone_kills_both_the_same_way`.

## Input classes checked with no mismatch

These were exercised and already agreed byte for byte; they are kept as
regression tests rather than recorded as defects.

* **Item counts**: 0 items (the `len == 0` early return in `call_fma`), 1, a
  few, 99, exactly 100 (loop exits on the bound rather than on a read failure),
  101, 150, 500, and 100 items followed by junk that is never read.
* **`scanf` failure paths**: junk as the very first token (prints 0), junk part
  way through, junk in the 100th position, and EOF at each of those points.
* **Sign handling**: bare `-`, bare `+`, `- 5`, `+-5`, `--5`, `5 -`, and `5-3`
  (which parses as `5` then `-3`).
* **Prefix-only numeric forms**: `0x10`, `0b101`, `3.7`, `1e5`, `1,2,3` — each
  parses a leading integer and then fails on the next call.
* **Whitespace**: `scanf` skips across newlines, so space, tab, newline, CR,
  vertical tab and form feed are interchangeable separators; leading and
  trailing runs of any of them, and whitespace-only input, all behave alike.
* **Range and overflow**: `INT_MAX`, `INT_MIN`, `2147483648`, `-2147483649`,
  `4294967296`, `LONG_MAX` and `LONG_MIN` neighbours, `2^64` neighbours,
  20-digit and 9000-digit magnitudes, and long leading-zero runs. glibc's
  conversion saturates at `LONG_MAX`/`LONG_MIN` and the result is truncated to
  `int`; the translation's saturating accumulator plus `as i32` cast reproduces
  this (e.g. `99999999999999999999` prints `-1`, `-99999999999999999999`
  prints `0`).
* **Non-text bytes**: invalid UTF-8 sequences, high bytes, and embedded NUL —
  the translation reads bytes and never assumes UTF-8.
* **Read-buffer boundaries**: whitespace padding of 4095/4096/8188–8194/16384
  bytes before a number or before junk, plus digit runs spanning the 8192-byte
  refill boundary, to confirm the one-byte pushback (`unget`) survives a buffer
  refill.
* **Volume**: 50,000 tokens on stdin (only the first 100 are consumed; the
  unread remainder is discarded by both).
* **Other fd states**: stdin as `/dev/null`. Manually checked as well: stdin
  closed outright and stdin pointing at a directory (both print `0`, exit 0),
  and stdout closed outright (`EBADF`, not a signal — both exit 0).
* **Randomized sweep**: 300 reproducible cases built from a fixed token
  alphabet (digits, signs, every whitespace character, junk words, overflow
  literals, `0xff`, NUL).

## Status

`cargo test` passes in both debug and release, 22 tests, none ignored or
skipped. Nothing in `c_src/` was modified.
