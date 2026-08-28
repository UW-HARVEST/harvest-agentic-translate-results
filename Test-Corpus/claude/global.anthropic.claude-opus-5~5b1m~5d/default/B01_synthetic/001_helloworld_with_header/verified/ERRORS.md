# Differential testing: mismatches found and fixed

Reference implementation: `c_src/` (ground truth, never modified).
Program under test: `translation/` (Rust).

Comparison method: both programs are built and run as **subprocesses**, and their
stdout, stderr and exit status are compared byte-for-byte
(`translation/tests/differential.rs`).

## What the C program does

`main()` takes no parameters and immediately returns `helloworld()`, which calls
`printf("Hello World!\n")` and returns `0`. There is no `scanf`, no `fgets`, no
`argc`/`argv` inspection, no length check, no null check and no early `return`.

The C is therefore **branch-free**: no input can steer it down a second path, and
there is no error path to reach. Consequently the enumerated input classes are not
"values the program parses" but the ways the process can be invoked and the ways
its single write can fail. Those are exactly where the mismatches turned up.

## Mismatch 1 — exit status when the reader of stdout hangs up (SIGPIPE)

**Status: found by test, fixed.**

| | C | Rust (before fix) |
|---|---|---|
| stdout | (empty, write failed) | (empty, write failed) |
| stderr | empty | empty |
| exit status | **killed by signal 13 (SIGPIPE)** | **exited 0** |

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` during runtime
startup, before `main` is entered. A C program launched from a shell inherits the
*default* disposition instead. So when the read end of stdout is closed:

- the C program's `printf`/flush raises `SIGPIPE` and the process is killed by it,
  reporting no exit code at all;
- the Rust program's write merely returned `EPIPE`, which the translation ignores
  (matching C's disregard for `printf`'s return value), and it exited 0.

This is the difference that a stdout-only test can never see: the bytes on stdout
are identical (both empty), and only the exit status diverges.

**Fix** (`translation/src/main.rs`): reset `SIGPIPE` to `SIG_DFL` via `signal(2)`
as the first thing `main` does, restoring C's process semantics. Declared as a
bare `extern "C"` so the crate keeps zero dependencies.

**Regression test.** `reader_hangs_up_early_same_disposition`. The pipe is created
by hand and its read end is closed *before* the child is spawned, so the child's
write is guaranteed to find no reader — dropping the read end after spawn would
race against the child's write and make the test flaky in both directions. The
test also asserts positively that the C side really was killed by signal 13, so it
cannot silently degrade into "both exited 0".

## Verified non-mismatches (behaviours that already matched, now locked down)

These were each a plausible way to get the translation wrong. They were checked
and already correct; tests now pin them.

### Write errors must be swallowed, not reported

`printf`'s return value is discarded by the C, so a failing write must not change
the output or the exit status. The translation uses
`let _ = handle.write_all(...)` rather than `println!`, which matters:

- `prog > /dev/full` (ENOSPC): C prints nothing to stderr and exits 0.
- `prog >&-` (fd 1 closed, EBADF): C prints nothing to stderr and exits 0.

A `println!`-based translation **panics** on either, emitting
`failed printing to stdout: No space left on device (os error 28)` on stderr and
exiting **101**. I confirmed this by temporarily mutating the source: the test
`stdout_write_error_enospc_is_ignored` catches it. Tests:
`stdout_write_error_enospc_is_ignored`, `stdout_closed_is_silent_and_still_exits_zero`.

### stdin is never read

Every stdin shape must be ignored: absent, empty, one item with no trailing
newline, a single line, 1000 lines, whitespace only, NUL and non-UTF-8 bytes,
numeric tokens spread across newlines (what `scanf("%d")` would consume but
`fgets` would not), a 100 000-character line, 1 MiB via a regular-file redirect,
1 MiB down a pipe the child never drains, and fd 0 closed outright.

The 1 MiB pipe case needs care in the *harness*, not the program: since neither
program reads stdin, writing the payload from the test's main thread would fill
the pipe buffer and deadlock against `wait`. The harness feeds stdin from a
separate thread and treats the resulting broken pipe as expected.

### argv is never inspected

`main()` declares no parameters, so all argv shapes print the greeting and exit 0:
none, one, 64 args, the empty string, whitespace-only args, flag-lookalikes
(`--help`, `-h`, `--version`, `-`, `--`, unknown flags), a 100 000-character arg,
and a non-UTF-8 arg (`\xff\xfe...`) that is legal for `execve` but not valid
Unicode.

### Formatting is not locale- or environment-sensitive

The greeting is a plain ASCII literal, so it must be byte-identical under an
empty environment and under `LC_ALL`/`LANG` of `C`, `C.UTF-8`, `en_US.UTF-8`,
`tr_TR.UTF-8` (the classic case-mapping trap) and `de_DE.UTF-8`. The working
directory is likewise irrelevant, since neither program touches the filesystem.

### Buffering mode does not change the byte stream

C stdio block-buffers to a pipe or regular file and line-buffers to a tty. Only
one write ever happens, so all three must produce the same bytes: verified for a
pipe, for a regular-file redirect, and under a real pty via `script(1)` (where
both produce `Hello World!\r\n`, the terminal's own NL→CRNL translation).

### Exact output shape

stdout is exactly the 13 bytes `Hello World!\n`: one trailing newline, no blank
line, no carriage return. Output is byte-identical across 20 repeated runs.

## Test suite integrity

29 tests, none `#[ignore]`d, skipped or disabled. The two tests that depend on
optional system facilities (`/dev/full`, `script(1)`) fall back to asserting the
ordinary differential case rather than silently passing when the facility is
absent.

The suite was mutation-tested to confirm it actually detects regressions. Each
mutation was applied to the Rust source, the suite was run, and the source was
restored:

| Mutation to the Rust source | Result |
|---|---|
| drop the trailing `\n` from the greeting | caught (many tests fail) |
| `return 1` instead of `0` | caught (many tests fail) |
| use `println!` instead of `write_all` | caught (`stdout_write_error_enospc_is_ignored`) |
| remove the `SIGPIPE` restoration | caught (`reader_hangs_up_early_same_disposition`) |

## Notes

- Nothing in `c_src/` was modified. The only addition under it is the generated
  `c_src/build/` directory produced by the documented CMake invocation.
- The test harness prefers that prebuilt `c_src/build/driver`; if it is absent it
  configures and builds the C program **out of tree**, into Cargo's target
  directory, so `cargo test` is self-contained without writing into `c_src/`.
