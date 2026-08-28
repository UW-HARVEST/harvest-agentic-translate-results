# Differential verification log — `c_src` vs `translation`

Ground truth: `c_src/src/main.c`

```c
int main() {
    printf("Hello World!\n");
    return 0;
}
```

* C binary: `c_src/build/driver` (built via `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`)
* Rust binary: `translation/target/release/driver` (`cd translation && cargo build --release`)
* Test suite: `translation/tests/differential.rs` — runs **both binaries as
  subprocesses** and compares stdout bytes, stderr bytes and exit status
  (including termination by signal) for every input class.

## Enumerated input classes

`main()` declares no parameters, never reads stdin, contains no conditionals and
has a single `return 0`. There is therefore no data-dependent branching to
cover: the reachable input classes are properties of the process environment
(argv shape, stdin shape, writability of stdout) rather than parsed values.

| # | Input class | C behavior | Status |
|---|---|---|---|
| 1 | no args, stdin = `/dev/null` | `Hello World!\n`, exit 0 | match |
| 2 | empty stdin (immediate EOF) | same | match |
| 3 | one line of stdin (`1\n`) | same (stdin never read) | match |
| 4 | stdin without trailing newline (`42`) | same | match |
| 5 | 1000 lines of stdin | same | match |
| 6 | 256 KiB of stdin (exceeds pipe buffer; writer cannot drain) | same | match |
| 7 | binary / non-UTF-8 stdin (all 256 byte values) | same | match |
| 8 | stdin closed outright (`exec 0<&-`) | same | match |
| 9 | one argv element | same (argv ignored) | match |
| 10 | 64 argv elements | same | match |
| 11 | flag-like args (`-h`, `--help`, `--version`) | same, exit 0 — **no usage text, no error** | match |
| 12 | empty-string arg, `-`, args containing spaces/newlines | same | match |
| 13 | unusual `argv[0]` (`""`, `weird-name`, `-bash`, bogus path) | same | match |
| 14 | stdout redirected to a regular file (C stdout fully buffered, flushed at exit) | file contains exactly `Hello World!\n` | match |
| 15 | stdout closed outright (`exec 1>&-`, write fails EBADF) | no output, **exit 0** | match |
| 16 | stdout = pipe with no reader (write fails EPIPE) | **killed by SIGPIPE** | **MISMATCH — fixed, see below** |
| 17 | stdout piped to a reader that discards input | `Hello World!\n`, exit 0 | match |
| 18 | stderr merged into stdout (`exec 2>&1`) | same, nothing on stderr | match |
| 19 | cleared environment (`env_clear`) | same | match |
| 20 | locale vars (`LC_ALL`/`LANG` = C, POSIX, en_US.UTF-8, tr_TR.UTF-8, de_DE.UTF-8) | same | match |
| 21 | 25 repeated runs | byte-identical every time | match |

## Mismatches found

### 1. SIGPIPE: Rust exited 0 where C is killed by signal 13

**Symptom.** With stdout connected to a pipe that has no reader, so the very
first write fails with `EPIPE`:

| | exit status |
|---|---|
| C `driver` | killed by `SIGPIPE` (signal 13; `128+13 = 141` as reported by a shell) |
| Rust `driver` (before fix) | `exit(0)` |

Reproduced with a FIFO whose only reader is closed before the program is
`exec`'d, which makes the failure deterministic rather than racy:

```sh
mkfifo fifo
exec 4<>fifo    # read-write open, so the next line does not block
exec 1>fifo     # stdout := write end
exec 4>&-       # close the only reader *before* the program runs
exec ./driver   # first write() now fails with EPIPE
```

Measured directly (C, the original Rust, and the fixed Rust):

```
C             (ground truth) : signal(13)
Rust PRE-fix  (no SIG_DFL)   : exit(0)
Rust POST-fix (current)      : signal(13)
```

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs; a C program inherits the default disposition (`SIG_DFL`). With the signal
ignored, the failing write returned an `Err(EPIPE)` that the translation
discarded, so the process ran to completion and returned 0 instead of dying from
the signal. This is invisible on the happy path and invisible to any test that
only compares stdout — the stdout of both programs is empty in this scenario;
only the exit status differs.

**Fix.** `translation/src/main.rs` restores the default `SIGPIPE` disposition as
the first action in `main`, via a direct `extern "C" { fn signal(...) }`
declaration (no new crate dependencies), guarded by `#[cfg(unix)]`. Write errors
are still discarded, which is correct: a `printf` failure does not change the
value C's `main` returns — confirmed by input class 15, where stdout is closed
outright (`EBADF`, not `EPIPE`) and both programs print nothing and exit 0.

**Regression test.** `stdout_is_pipe_with_no_reader_sigpipe`. It asserts the
premise (the C program really is killed by signal 13 in this scenario) before
comparing, so the test cannot silently degrade into a vacuous pass if the
scenario ever stops producing `EPIPE`.

## Test-harness defects found and fixed (not translation bugs)

These are recorded because each one produced a misleading test result:

1. **Vacuous SIGPIPE premise.** The first version of the SIGPIPE test asserted C
   exited with code `141`. Because the helper script ends in `exec`, the program
   *replaces* the shell, so the signal is observed directly as `signal(13)`
   rather than being translated by an intermediate shell into `128+13`. The
   assertion was corrected to `signal(13)`; comparison is done on a status
   string that renders signal deaths distinctly from exit codes, so a
   signal-vs-exit-code difference can never compare equal.
2. **Concurrent cmake invocations.** Integration tests run as threads in one
   process. With the C binary absent, all 21 tests invoked `cmake` in the shared
   `c_src/build` directory at once and clobbered each other's temporaries; the
   configure step then failed with "the C compiler is broken" and all 21 tests
   failed for a reason unrelated to the translation. The C build now happens
   exactly once behind a `OnceLock`.
3. **Missing `mkfifo` in a scratch reproduction.** An early manual check
   redirected stdout to a path that was a *regular file* rather than a FIFO, so
   the write succeeded and the C program appeared to exit 0 — hiding the very
   mismatch being investigated. Worth noting because the SIGPIPE scenario fails
   silently (looks like a pass) if the FIFO is not actually created; the test
   therefore asserts `mkfifo` succeeded and that C really died of signal 13.

## Notes on faithfulness

* Output is written as raw bytes (`write_all(b"Hello World!\n")`), so the exact
  13 bytes of `printf`'s output are reproduced with no formatting layer that
  could alter spacing or the trailing newline.
* `main` returns `ExitCode::from(0)`, mirroring `return 0`.
* Neither program reads stdin; large/binary stdin is left unconsumed by both.
* Nothing in `c_src/` was modified. The only additions there are the generated
  `c_src/build/` artifacts produced by the prescribed cmake build.

## Result

Both programs build without errors. All 21 input classes produce identical
stdout, stderr and exit status. `cargo test` and `cargo test --release` pass
(21 tests, 0 failures, 0 ignored); no test is disabled, skipped or `#[ignore]`d.
