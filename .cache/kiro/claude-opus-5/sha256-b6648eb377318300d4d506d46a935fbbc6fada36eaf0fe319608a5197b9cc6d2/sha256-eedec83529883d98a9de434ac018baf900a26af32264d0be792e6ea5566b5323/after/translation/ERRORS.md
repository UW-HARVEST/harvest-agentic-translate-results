# Differential verification of `c_src/src/main.c` vs `translation/`

The C program reads one byte with `getchar()`, stores it in a `char`, and prints
14 lines: 12 `<ctype.h>` classification results with `%d`, then `tolower` and
`toupper` with `%c`.

## How it was checked

- C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
- Rust: `cd translation && cargo build --release`
  → `translation/target/release/driver`
- `translation/tests/differential.rs` runs both binaries as subprocesses and
  compares stdout, stderr and exit status (including death by signal). The
  input space is exhaustive: all 256 possible byte values, plus empty input,
  plus non-byte-valued conditions (multi-byte stdin, arguments, failing stdin,
  broken/closed/file stdout).

## Mismatches found

### 1. Exit status on a broken stdout pipe (fixed)

| | C | Rust (before fix) |
|---|---|---|
| stdout with no reader | killed by `SIGPIPE`, status 141 | exited 0 |

**Cause.** The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main`, so writes
to a pipe with no reader return `EPIPE` instead of terminating the process. The
translation discarded write errors (correctly, mirroring C's stdio, which does
not report them), so the process ran to a clean exit. The C program inherits the
default `SIGPIPE` disposition and dies from signal 13.

**Fix.** `reset_sigpipe()` in `src/main.rs` calls `signal(SIGPIPE, SIG_DFL)` at
the top of `main`. `signal` is declared in a local `extern "C"` block; libc is
already linked on the gnu targets, so no new dependency was added.

**Test.** `broken_stdout_pipe_exit_status`. It hands the child a stdout pipe
whose read end is closed and asserts both programs die from signal 13. Two
details make it deterministic rather than racy:

- the pipe is created with `pipe2(O_CLOEXEC)`. With plain `pipe()` the child
  inherits the read end of its own stdout pipe, keeps a reader alive, and never
  sees a broken pipe — the first version of this test failed for exactly that
  reason and reported C exiting 0.
- the read end is closed while the child is still blocked reading stdin, so the
  pipe is guaranteed broken before the child's first write.

## Behaviours verified as already correct

These are the places the translation could plausibly have diverged. Each was
confirmed against the C binary, not reasoned about in the abstract.

- **`is*` return the masked class bit, not `1`.** glibc's `ctype.h` defines
  `isalpha(c)` as a `__ctype_b` table lookup masked with `_ISalpha`, so
  `printf("%d", isalpha('a'))` prints `1024`. The translation reproduces every
  `_ISbit` value: upper 256, lower 512, alpha 1024, digit 2048, xdigit 4096,
  space 8192, print 16384, graph 32768, blank 1, cntrl 2, punct 4, alnum 8.
  A run on `'a'` prints `alphabetic: 1024`, `graphical: 32768`, `alphanumeric: 8`.
- **Bytes `0x80`–`0xFF`.** `char` is signed here, so these become negative
  indices into `__ctype_b`. glibc's table covers `-128..=255` and the C locale
  leaves the high half all-zero with identity case mapping, so every class
  reports `0`. Signedness turns out to be unobservable: the unsigned reading
  (indices 128–255) gives the same answers.
- **EOF.** `char c = getchar()` truncates `EOF` to `(char)-1`, which indexes the
  entry for `0xFF`: all classes `0`, and `%c` of `tolower(-1)` emits a raw
  `0xFF` byte. Empty stdin, a closed fd 0, and a directory as stdin all take
  this path identically in both programs.
- **`%c` writes a raw byte.** `printf("%c", ...)` converts to `unsigned char`
  and writes one byte, never UTF-8. For input `0xFF` the last line is
  `to upper: \xff\n` and stdout is not valid UTF-8;
  `high_byte_output_is_raw_not_utf8` asserts this so the Rust side cannot
  silently start encoding `char` values.
- **Only the first byte is consumed.** A leading `\n` is returned as data rather
  than skipped, and trailing input is ignored (checked up to 100 000 bytes).
- **Arguments are ignored.** `main` takes no parameters.
- **Closed stdout (fd 1 closed).** Writes fail with `EBADF`; C's stdio swallows
  it and still exits 0 with empty stderr. Both match.
- **Buffering.** C's stdout is fully buffered to a file and line buffered to a
  terminal; total output is ~200 bytes either way and stderr is never written,
  so the byte stream is identical. Confirmed with stdout redirected to a file.
- **Exit code.** `main` falls off the end, so a successful run exits 0.

## Test-suite integrity

The suite was mutation-checked to confirm it is not vacuous. Each of these
changes to `src/main.rs` was reverted after confirming it caused a failure:

| Mutation | Caught by |
|---|---|
| remove `reset_sigpipe()` | `broken_stdout_pipe_exit_status` |
| `ALNUM` mask `8` → `1` | `every_byte_value`, `class_range_boundaries`, +4 |
| `%c` written as a UTF-8 `char` | `every_byte_value`, `only_first_byte_is_consumed`, +5 |
| EOF returns `0` instead of `-1` | `empty_input_eof_path`, both stdin-failure tests |

## Status

- both programs build without errors
- 12 tests, `cargo test` passes in debug and release; none disabled, skipped or
  `#[ignore]`d
- all 261 enumerated inputs produce identical stdout, stderr and exit status
- nothing in `c_src/` was modified
