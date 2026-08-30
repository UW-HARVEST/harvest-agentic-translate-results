# Differential testing report: C (`c_src`) vs Rust (`translation`)

The C program is the ground truth. Both binaries are run as subprocesses and
compared on **stdout (bytes)**, **stderr (bytes)**, **exit code**, and
**terminating signal**.

## How to build and run each program

```bash
# C reference
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
# -> ./c_src/build/driver

# Rust translation
cd translation && cargo build --release
# -> ./translation/target/release/driver

# Differential suite
cd translation && cargo test
```

## Reference output (identical for every input class below)

```
Calling good()...
good()
helperGood()
Finished good()
Calling bad()...
bad()
Finished bad()
```

exit code 0, empty stderr.

---

## Mismatches found and fixed

### 1. Broken stdout pipe: C died from SIGPIPE, Rust exited 0 — FIXED

**Severity: real behavioral divergence in exit status.**

| | C | Rust (before fix) |
|---|---|---|
| terminating signal | `SIGPIPE` (13) | none |
| shell exit status | 141 | 0 |

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs. A C program inherits the default disposition, which terminates the
process. So when stdout was a pipe with no reader, the C program was killed by
`SIGPIPE` while the Rust program's `write` merely returned `EPIPE`, which the
translation discarded via `let _ = ...`, and then exited 0.

**Fix** (`src/main.rs`): restore the default handler at the top of `main`, using
`signal()` from the already-linked libc — no new dependency:

```rust
extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;
fn restore_default_sigpipe() { unsafe { signal(SIGPIPE, SIG_DFL); } }
```

**Test.** `broken_stdout_pipe_matches_including_sigpipe`.

Note on making this test *deterministic*: my first attempt raced a FIFO reader
that exited immediately, and it was flaky — the C program produced 141 on one
run and 0 on the next. Because this program emits only ~103 bytes, which fits
entirely in the 64 KiB pipe buffer, the write succeeds unless the read end is
*already* gone. The test therefore creates a pipe with `pipe(2)`, hands the
**write end** to the child as stdout, and closes the **read end before
spawning**. The first write then always fails with `EPIPE`.

The test was verified to actually catch the bug: with the fix commented out it
fails with
`broken pipe: terminating signal differs (C=Some(13), Rust=None)`.
With the fix, C and Rust both report signal 13.

### 2. stdout buffering discipline did not match C stdio — FIXED (hardening)

**Severity: not observable through normal shell use; fixed for fidelity.**

The original translation called `write_all` twice per line (14 write syscalls).
C stdio buffers stdout and, when stdout is **not** a terminal, flushes once at
exit (a single ~103-byte write); when stdout **is** a terminal it is
line-buffered.

No input was found where this changed the compared bytes, because the whole
output fits in a pipe buffer, so every chunking pattern succeeds. It is however
observable in principle: if a pipe's reader vanished mid-run, the C program
(one write at exit) and the Rust program (14 writes) could die at different
points and emit different amounts of partial output.

**Fix.** `src/main.rs` now routes output through a small `CStdout` buffer that
replicates C's discipline: line-buffered when `isatty(1)`, otherwise fully
buffered and flushed once as `main` returns. Write errors are still ignored,
matching C's unchecked `printf`/`fflush`.

Verified both modes: the pipe/file cases in the suite, plus a manual run of both
binaries under a real pseudo-terminal (`script -qec`), whose output was
byte-identical (md5 `e3ddb56b921673c2f612c8aaad995f0d`).

---

## Faithfully preserved C quirks (deliberately NOT "fixed")

- **`bad()` does not call `helperBad()`.** `good()` calls `helperGood()`, but
  `bad()` only prints `"bad()"`. `helperBad()` is defined and never referenced
  anywhere. The output is asymmetric as a result — `helperGood()` appears, and
  `helperBad()` never does. The Rust keeps `helper_bad` defined (behind
  `#[allow(dead_code)]`) and never calls it.
- **`printLine`'s `NULL` guard is unreachable.** All 8 call sites pass string
  literals, so the `if (line != NULL)` false-arm can never be taken from the
  executable. Kept as `Option<&str>` for structural fidelity; it cannot be
  reached by any input, so no test can cover it.
- **`argc`/`argv` are accepted and completely ignored**; there is no argument
  parsing, no usage message, and no error path.
- **Write errors are never checked.** `main` has a single `return 0`, so the
  program exits 0 even when output cannot be written (e.g. closed fd 1,
  `/dev/full`).

## Input classes enumerated and covered

The program reads no stdin (no `scanf`/`fgets`/`getchar`) and ignores `argv`,
and its only conditional is the unreachable `NULL` check. Its single `return 0`
means there are **no error-exit paths**. The observable input space is therefore
argv shape, stdin content, the state of the standard file descriptors, and the
environment:

| # | Input class | Test |
|---|---|---|
| 1 | no args, empty stdin (happy path) | `no_args_empty_stdin` |
| 2 | exact expected bytes pinned for C | `c_output_is_the_documented_bytes` |
| 3 | exact expected bytes pinned for Rust | `rust_matches_exact_expected_bytes` |
| 4 | single arg | `single_arg` |
| 5 | empty-string arg | `empty_string_arg` |
| 6 | flag lookalikes (`-h`, `--help`, `-`, spaces) | `several_args_including_flag_lookalikes` |
| 7 | 256 args | `many_args` |
| 8 | non-UTF-8 bytes + unicode args | `non_utf8_and_unicode_args` |
| 9 | 100 000-char arg | `very_long_single_arg` |
| 10 | unusual `argv[0]` | `unusual_argv0_matches` |
| 11 | stdin with text (must stay unconsumed) | `stdin_with_text_is_ignored` |
| 12 | stdin without trailing newline | `stdin_without_trailing_newline` |
| 13 | stdin with NUL/binary bytes | `stdin_binary_and_nul_bytes` |
| 14 | 1 MiB stdin | `stdin_large` |
| 15 | stdin closed (fd 0 absent) | `stdin_closed_entirely` |
| 16 | **stdout pipe with no reader → SIGPIPE** | `broken_stdout_pipe_matches_including_sigpipe` |
| 17 | stdout closed (fd 1 absent) | `closed_stdout_matches` |
| 18 | stderr closed (fd 2 absent) | `closed_stderr_matches` |
| 19 | stdout to a regular file (block buffered) | `stdout_to_regular_file_matches` |
| 20 | stdout to `/dev/full` (every write ENOSPC) | `stdout_to_dev_full_matches` |
| 21 | varying locale env vars | `locale_env_does_not_change_output` |
| 22 | empty environment | `empty_environment` |
| 23 | different working directory | `different_cwd_matches` |
| 24 | repeated runs (determinism) | `repeated_runs_are_stable` |

`stdout_to_dev_full_matches` returns early if `/dev/full` does not exist on the
platform; it is not `#[ignore]`d, and on Linux (where these results were
produced) it executes the comparison.

## Final status

- Both programs build with no errors and no warnings.
- `cargo test` — **25 passed, 0 failed, 0 ignored** (verified in both the debug
  and `--release` profiles).
- No test is disabled, skipped or `#[ignore]`d.
- Nothing in `c_src/` was modified (only the required `c_src/build/` output).
