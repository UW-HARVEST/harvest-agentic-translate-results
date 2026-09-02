# Differential testing: mismatches found and fixed

The C program (`c_src/src/main.c`) is the ground truth. Both binaries were built
and run as subprocesses on identical stdin, and stdout, stderr and exit status
were compared byte for byte (`translation/tests/differential.rs`).

## What the C program does

`main` calls `scanf("%d", &x)` with `x` initialised to `0`, then branches on
`if (x)`:

- `x != 0` -> `good()` -> `goodG2B()` prints `04`, then `goodB2G()` evaluates
  `data < CHAR_MAX/2` as `127 < 63`, which is false, so it prints
  `data value is too large to perform arithmetic safely.`
- `x == 0` -> `bad()` -> `data = CHAR_MAX` (127), `data * 2` is computed in `int`
  (254) and truncated back into a signed `char`, wrapping to `-2`.
  `printf("%02x\n", ...)` promotes that `char` to `int` and `%x` reinterprets the
  bits as `unsigned int`, so the output is `fffffffe` -- eight hex digits, not
  two, because `02` is only a *minimum* field width.

Exit status is always `0` on any input, and neither program ever writes to
stderr. Both reachable outputs are pinned to literal bytes in the test suite so
the two programs cannot drift together unnoticed.

## Mismatches found

### 1. Panic/abort instead of ignoring a stdout write failure

- **Symptom.** With stdout pointed at `/dev/full` (every write fails with
  `ENOSPC`), the C exited `0` with empty stderr. The Rust aborted with status
  `134`, printing `thread 'main' panicked ... failed printing to stdout: No space
  left on device` to stderr. Both stderr and exit status differed.
- **Cause.** The translation used `println!`, which panics when the underlying
  write fails; with `panic = "abort"` in the release profile that becomes
  `SIGABRT`. C's `printf` reports failure only through its return value and
  `errno`, and this program ignores both; a failed flush inside `exit` likewise
  does not change the status `main` returned.
- **Fix.** Output now goes through an explicit buffer written once with
  `write_all`, and both the write and the flush results are discarded
  (`let _ = ...`). See `flush_stdout` in `src/main.rs`.
- **Covered by.** `stdout_write_error_is_ignored`.

### 2. Wrong exit status on a broken stdout pipe

- **Symptom.** With stdout on a pipe whose reader had closed, the C died from
  `SIGPIPE` (signal 13, shell status 141). The Rust exited 134 with a panic
  message on stderr; after fix 1 alone it would have exited `0`. Either way the
  status differed.
- **Cause.** The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, so
  a write to a broken pipe returns `EPIPE` instead of killing the process. A C
  program inherits the default disposition, which terminates it.
- **Fix.** `restore_default_sigpipe()` resets `SIGPIPE` to `SIG_DFL` as the first
  statement of `main`.
- **Covered by.** `broken_stdout_pipe_matches`. The test closes the read end
  while the child is still blocked in `scanf` and only then sends EOF on stdin,
  so the child cannot write before the pipe is broken -- the test is not racy.

### 3. stdio buffering discipline (fixed together with 1)

- `println!` writes through a `LineWriter`, flushing at every newline, whereas
  glibc block-buffers a non-tty stdout and emits everything in a single `write`
  at exit. This is invisible when stdout is captured in full, but it changes what
  is observable when a write fails partway through. Buffering all output and
  writing once at exit reproduces C's syscall pattern as well as its bytes.

## Behaviours checked and already correct

These were verified rather than fixed; they are the places a translation of this
program is most likely to go wrong.

- **`scanf` crosses newlines.** Leading whitespace is skipped using C's
  `isspace` set (space, `\t`, `\n`, `\v`, `\f`, `\r`), so `"\n\n\n  7"` yields 7.
  `fgets` would have stopped at the first newline.
- **Matching failure leaves `x` untouched.** `""`, `"abc"`, `"-"`, `"+"`, `"--1"`,
  `"- 1"`, `".5"`, whitespace only, a leading NUL byte, and invalid UTF-8 all
  leave `x` at its initialiser `0` and therefore reach `bad()`. The Rust
  `scanf_int` returns `None` and the caller does not assign.
- **`int` truncation of the converted value.** glibc converts `%d` with `strtol`,
  which saturates at `long` bounds, and then stores the result into an `int`,
  keeping the low 32 bits. So `4294967296` (2^32), `68719476736` (2^36),
  `-4294967296`, `-9223372036854775808` (`LONG_MIN`), `-9223372036854775809`
  (saturates to `LONG_MIN`) and `-99999999999999999999` all truncate to `0` and
  reach `bad()`, while `2147483648`, `9223372036854775808` (saturates to
  `LONG_MAX`, low 32 bits `0xffffffff`) and `18446744073709551616` reach
  `good()`. Clamping to `int` bounds instead of truncating is a real divergence
  and is caught by `int_truncation_of_the_converted_value`.
- **Conversion stops at the first non-digit.** `"0x10"` is 0 (no hex prefix for
  `%d`), `"3.9"` is 3, `"1foo"` is 1, `"  12  34"` is 12.
- **Signed `char` overflow is preserved, not corrected.** `bad()` still wraps
  127*2 to -2; the dead store `data = ' '` in `goodB2G()` is kept.
- **`printLine`'s NULL check** is unreachable from the C's only call site, which
  passes a string literal.
- **argv is ignored**; `main` takes no parameters.

## Harness validation

To confirm the suite is not vacuous, five deliberate defects were injected into
`src/main.rs` one at a time; each was caught, and the source was restored and
re-verified byte-identical afterwards.

| Injected defect | Tests that failed |
| --- | --- |
| `char_hex as u8` instead of promoting to `int` (prints `fe`, not `fffffffe`) | 9 tests, incl. `golden_output_bytes_are_what_the_c_produces` |
| whitespace set narrowed to space and tab (newlines no longer skipped) | `scanf_skips_whitespace_across_lines`, `every_leading_byte_agrees` |
| clamp to `int` bounds instead of truncating the low 32 bits | `int_truncation_of_the_converted_value`, `maximum_sized_inputs`, `stderr_is_always_empty_and_status_always_zero` |
| `unwrap()` on the stdout write | `stdout_write_error_is_ignored` |
| `restore_default_sigpipe()` removed | `broken_stdout_pipe_matches` |

## Status

- Both programs build with no errors or warnings.
- `cargo test` and `cargo test --release`: 16 passed, 0 failed, 0 ignored.
- No test is disabled, skipped or `#[ignore]`d.
- Nothing in `c_src/` was modified; only the `c_src/build/` output directory was
  created, by the build commands.
