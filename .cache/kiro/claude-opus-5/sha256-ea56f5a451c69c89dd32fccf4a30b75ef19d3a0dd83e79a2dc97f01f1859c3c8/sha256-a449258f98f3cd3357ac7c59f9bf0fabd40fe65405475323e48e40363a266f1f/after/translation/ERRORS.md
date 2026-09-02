# Differential verification notes

C ground truth: `c_src/src/main.c` — `int x = 0; scanf("%d", &x); driver(x);`
where `driver` memcpy's the object representation of the `int` and prints its
`sizeof(int)` bytes with `%02x`, followed by `"\n"`.

Rust under test: `translation/src/main.rs`, driven as a subprocess by
`translation/tests/differential.rs`. Every case compares stdout, stderr and
exit status.

## Mismatches found

### 1. Broken stdout pipe: exit 0 instead of death by SIGPIPE

- **Symptom**: with stdout connected to a pipe whose reader has already exited,
  the C program is terminated by `SIGPIPE` (shell status 141) while the Rust
  program exited 0. stdout and stderr were identical (both empty), so a
  stdout-only comparison passed and hid the difference.

  ```
  $ { sleep 0.5; echo 42; } | c_src/build/driver | true      # -> 141
  $ { sleep 0.5; echo 42; } | translation/.../driver | true  # -> 0   (wrong)
  ```

- **Cause**: the Rust standard library sets `SIGPIPE` to `SIG_IGN` before
  entering `main`. A failing write therefore returns `EPIPE` instead of killing
  the process, and the translation discards write errors (`let _ = write!(...)`)
  exactly as C's `printf` return value is discarded — so the program ran to
  completion and returned 0. The C program keeps the default disposition, so the
  same write kills it with signal 13.

- **Fix**: `restore_default_sigpipe()` in `src/main.rs` resets `SIGPIPE` to
  `SIG_DFL` (via `extern "C" signal`) as the first statement of `main`, under
  `#[cfg(unix)]`.

- **Regression test**: `broken_stdout_pipe_matches`. Verified to have power —
  with the `restore_default_sigpipe()` call commented out the test fails with
  `C Ok(141) vs Rust Ok(0)`; with it restored it passes and both sides report
  141 (so the test is not vacuously comparing 0 to 0).

## Behaviors confirmed to already agree

These were candidate mismatches that testing showed the translation already
reproduces. Recorded so a later reader does not have to rediscover them.

- **Input failure**: EOF, `/dev/null` stdin, a closed stdin fd, and
  whitespace-only input all leave `x` at its initializer `0`, printing
  `00000000`. `scanf` does not write to the destination on failure and the C
  program ignores the return value.
- **Matching failure**: a first non-whitespace byte that cannot start a number
  (`abc`, `.`, `-`, `+`, `--5`, `-  5`, …) also leaves `x` at `0`.
- **Whitespace class**: `%d` skips exactly the `isspace` set
  (`' ' \t \n \v \f \r`) and nothing else — a UTF-8 thin space is a matching
  failure, not whitespace.
- **Truncation past `int`**: glibc converts `%d` into a `long` and stores the low
  bytes, so `2147483648` prints `00000080` and `4294967296` prints `00000000`
  rather than saturating at `INT_MAX`. The Rust model reproduces this with
  `value as i32`.
- **Clamping past `long`**: on range error glibc yields `LONG_MAX` / `LONG_MIN`
  before the store, so `9223372036854775808` and `"9"*10000` both print
  `ffffffff`, and `-9223372036854775809` and `-"9"*10000` both print `00000000`.
  Leading zeros do not trigger this (`"0"*10000 + "5"` prints `05000000`).
- **Early stop**: conversion ends at the first non-digit; `0x1f` reads `0`,
  `1e5` reads `1`, `12abc` reads `12`.
- **Reads across newlines**: `scanf` consumes leading newlines and stops after
  the first field; trailing lines are never read.
- **Byte layout**: `memcpy` of the `int` yields little-endian bytes on this
  target, matching `i32::to_ne_bytes`.
- **Non-text input**: NUL bytes and invalid UTF-8 are handled identically; the
  Rust side reads raw bytes and never decodes them as UTF-8.
- **Output format**: lowercase `%02x` per byte, four bytes, one trailing `\n`,
  nothing on stderr, exit code 0 on every normal path.

## Coverage

`translation/tests/differential.rs`, 22 tests, no `#[ignore]`, all passing.
Includes all 256 single-byte inputs, all 256 two-byte combinations over the
byte classes `%d` distinguishes, every bit position of the printed `int`, and
10 000-byte digit/whitespace/junk runs.

Additionally fuzzed outside the suite: 4 800 randomized inputs (random bytes
from the interesting alphabet, random raw bytes, and structured numeric strings)
produced zero differences in stdout, stderr or exit status.
