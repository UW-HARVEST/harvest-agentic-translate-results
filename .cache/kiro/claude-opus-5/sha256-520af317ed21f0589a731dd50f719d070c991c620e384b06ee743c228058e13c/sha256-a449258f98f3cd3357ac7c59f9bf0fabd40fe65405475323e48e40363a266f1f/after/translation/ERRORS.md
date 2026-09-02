# Differential findings: `c_src/src/main.c` vs. `translation/`

The C program is the ground truth. Everything below describes a divergence in
the **Rust** side and how the Rust side was changed to match. `c_src/` was never
modified (its sources still carry their original checkout mtime; only
`c_src/build/` artifacts were added by the prescribed CMake build).

## What the C program actually does

```
Calling good()...
0
2
Calling bad()...   <- preceded by "Finished good()"
0
0
Finished bad()
```

74 bytes on stdout, nothing on stderr, exit status 0.

Two properties worth stating explicitly, because both look like bugs and both
are reproduced deliberately rather than "fixed":

* `bad()` evaluates `intOne + intTwo;` as a bare expression statement and throws
  the result away, so `intSum` stays `0` and `bad()` prints `0` twice. The
  translation keeps this (`let _ = int_one + int_two;`).
* `printLine` guards on `line != NULL`, but every call site passes a string
  literal, so the guard never rejects anything. It is retained as
  `Option<&str>` to keep the control flow shape.

The program never reads stdin and never inspects `argc`/`argv`. There is
therefore no input-dependent control flow at all; the only behaviour that can
diverge lives in the I/O environment. That is where both real mismatches were.

---

## Mismatch 1 — broken stdout pipe: exit 0 instead of death by SIGPIPE

**Severity: real, and invisible to any stdout-only test.**

The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main` runs. A C
program inherits the default disposition. So when stdout is a pipe whose reader
has gone away:

| | exit status |
|---|---|
| C | killed by signal 13 (`SIGPIPE`) |
| Rust (before fix) | exited 0 |

Reproduced by creating a pipe, closing the read end, and passing the write end
as the child's stdout, so the very first write hits `EPIPE`:

```
c_src/build/driver:                  returncode=-13
translation/target/release/driver:   returncode=0     <- mismatch
```

**Cause.** Rust runtime start-up code masks `SIGPIPE`; the previous translation
additionally swallowed the write error with `let _ = writeln!(...)`, so nothing
was left to observe.

**Fix.** Restore the default disposition at the top of `main`:

```rust
unsafe { signal(SIGPIPE, SIG_DFL); }
```

`signal` is declared in a small `extern "C"` block rather than by adding the
`libc` crate, since Rust already links the C runtime on Linux.

**Guarded by** `broken_stdout_pipe_kills_both_with_sigpipe`. Verified to be a
live test by negative control: deleting the `signal` call makes it fail
(`broken-pipe mismatch`), and restoring it makes it pass.

---

## Mismatch 2 — stdout buffering discipline did not match glibc

**Severity: not observable through stdout/stderr/exit status for this program.
Corrected anyway for faithfulness; see the honest caveat below.**

glibc chooses the buffering mode for `stdout` from what fd 1 is:

* a TTY -> line buffered, one write per `\n`;
* anything else (pipe, file, closed fd) -> fully buffered, so this program's
  whole 74-byte output leaves in a single `write` at exit.

Rust's `io::stdout()` is a `LineWriter` unconditionally, so the previous
translation issued one write per line even when stdout was a pipe. Combined with
the fix for mismatch 1, that difference is what decides who gets killed when a
reader stops reading partway through: the C program has already handed over all
74 bytes in one successful write, whereas a line-buffered writer can be killed
between lines.

**Fix.** `CStdout` in `src/main.rs` emulates the glibc discipline: it picks the
mode from `isatty(1)` once, accumulates into a `Vec`, flushes on `\n` only in
line-buffered mode, flushes on a full buffer (`BUFSIZ`, 4096), and flushes at
exit the way stdio does when `main` returns.

**Honest caveat, established by negative control.** Forcing the translation back
to line buffering (`let line_buffered = true;`) leaves the *entire* suite green.
For this program the buffering mode is genuinely unobservable through the three
graded channels, because:

* the output is 74 bytes, far below the 64 KiB pipe capacity, so no write ever
  blocks or is split;
* the program finishes in microseconds, so a reader cannot realistically close
  between two writes;
* on `/dev/full` and on a closed fd 1, every write fails either way and C
  ignores all the return values, so the status is 0 regardless;
* on a TTY the terminal applies ONLCR to the same bytes in either mode.

So `reader_closing_early_matches` does **not** prove what an earlier version of
its comment claimed; the comment was corrected rather than left overstating the
evidence. The emulation is kept because it is what glibc does, and it removes
the divergence in write granularity that only a syscall trace would expose.

---

## Error paths checked and found already matching

These reach C's failure paths but needed no change, because C checks neither
`printf`'s nor `fflush`'s return value, so a failed write cannot alter the exit
status:

| Input class | C | Rust | Test |
|---|---|---|---|
| stdout to `/dev/full` (every write `ENOSPC`) | exit 0, no stderr | same | `stdout_write_failure_on_dev_full` |
| stdout closed, `>&-` (every write `EBADF`) | exit 0, no stderr | same | `stdout_closed` |
| stdin closed, `<&-` | exit 0 | same | `stdin_closed` |
| stdout to a real PTY (line-buffered branch, CRLF) | 82 bytes w/ CRLF, exit 0 | same | `stdout_on_a_tty_matches` |
| stdout to a regular file | identical bytes | same | `stdout_redirected_to_file_matches` |

## Input classes enumerated and covered

Since the C reads no stdin and ignores argv, "inputs" are argv shapes, stdin
shapes, and the stdout target. All are asserted on stdout **and** stderr **and**
exit status:

* argv: none; one; several; empty string; flag-like (`-h`, `--help`,
  `--version`); numeric incl. `2147483647` / `-2147483648`; embedded whitespace
  and newline; non-ASCII/emoji; path traversal; `printf` format specifiers
  (`%s %d %n %x`); a 4096-byte argument; 256 arguments.
* stdin: `/dev/null`; empty; one line; a single item with no trailing newline;
  several lines; non-numeric; binary incl. NUL and `0xFF`; whitespace only;
  1 MiB (larger than a pipe buffer); fd 0 closed.
* stdout target: pipe, regular file, PTY, `/dev/full`, closed fd, pipe with a
  dead reader, pipe with a reader that leaves early.
* environment and working directory varied, and determinism checked over 10
  consecutive runs.

## Verification status

* Both programs build with no errors; `cargo build --release` and
  `cargo clippy --release` are warning-free.
* `cargo test` passes: 15 tests, 0 failed, 0 ignored. No `#[ignore]`, no
  skips, no disabled cases.
* Every test spawns both binaries as subprocesses and compares stdout bytes,
  stderr bytes, and exit status (distinguishing a normal exit code from death by
  signal). Nothing loads the Rust code as a library.
