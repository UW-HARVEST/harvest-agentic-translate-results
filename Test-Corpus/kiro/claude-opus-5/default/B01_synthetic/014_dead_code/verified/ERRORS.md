# Differential testing: mismatches found and their causes

Ground truth is `c_src/src/main.c`. Both programs are compared by running them
as subprocesses and diffing stdout, stderr and exit status (including
termination signal). Tests live in `tests/differential.rs`.

## What the C program actually does

`main` is straight-line code with no data-dependent branches:

```
printLine("Calling good()...");  good();  printLine("Finished good()");
printLine("Calling bad()...");   bad();   printLine("Finished bad()");
return 0;
```

It never reads stdin, never inspects `argc`/`argv`, and reads no environment
variables. Its output is a fixed 92 bytes and its exit status is always 0.
So the input space that can change behavior is not the argument vector or
stdin — it is **the state of the standard file descriptors**, which is where
both real mismatches were found.

Branch inventory, and how each is reached:

| Location | Branch | Reachable? | Covered by |
| --- | --- | --- | --- |
| `printLine` | `line != NULL` true | yes, all 7 calls | every test |
| `printLine` | `line == NULL` false-branch (no output) | **no** — every caller passes a string literal | structurally unreachable; preserved as `Option::None` in the translation |
| `helperBad` | whole function | **no** — `static`, never called | `helper_bad_is_never_called` |
| `helperGood` | whole function | yes, via `good()` | every test |
| `main` | — | no branches | — |

## Mismatch 1 — broken stdout pipe: C died from SIGPIPE, Rust exited 0

**Symptom.** With stdout on a pipe whose read end was already closed:

| | exit status |
| --- | --- |
| C | killed by signal 13 (`SIGPIPE`); shell reports 141 |
| Rust (before fix) | exited 0 |

stdout and stderr agreed (both empty); only the status differed. A test that
checked stdout alone would have passed.

**Cause.** Two things compounding:

1. Rust's standard library sets `SIGPIPE` to `SIG_IGN` before `main` runs. The
   C program inherits the default disposition. So the failing write raised no
   signal in the Rust program and returned `EPIPE` instead.
2. The translation discarded that error with `let _ = out.write_all(...)`, so
   the `EPIPE` went nowhere and `main` fell through to `exit(0)`.

**Fix.** `reset_sigpipe()` in `src/main.rs` restores `SIG_DFL` for `SIGPIPE` at
the top of `main`, via a direct `extern "C"` declaration of `signal` (no new
dependency for one symbol). Discarding the write error is *correct* and was
kept — see mismatch 3 — the defect was purely the signal disposition.

**Regression-checked.** Commenting out `reset_sigpipe()` makes
`broken_stdout_pipe_matches` fail and leaves the other 28 tests passing, so the
test is genuinely load-bearing.

## Mismatch 2 — stdout buffering discipline (latent, not observable here)

**Cause.** glibc gives `stdout` *full* buffering when it is not a terminal and
*line* buffering when it is. `std::io::Stdout` is always a `LineWriter`. The C
program therefore emits its entire output in a single 92-byte write at exit when
stdout is a pipe or file, whereas the original translation emitted six separate
writes.

**Fix.** The `Out` writer in `src/main.rs` reproduces glibc's rule: it checks
`isatty(1)`, flushing per newline on a terminal and otherwise buffering
everything until the exit-time flush.

**Honest scope.** No deterministic test in this suite distinguishes the two
disciplines, and I verified that: forcing `line_buffered: true` unconditionally
still leaves all 29 tests green. The reason is that the total output (92 bytes)
is far smaller than any pipe buffer, so the final byte sequence is identical
either way, and the write *count* is only observable to a reader that consumes
part of the stream and then closes — which is inherently racy to assert. In the
broken-pipe test the reader is gone before the first write, so both disciplines
deliver zero bytes and die identically. This change is recorded as fidelity to
the C's I/O model rather than as a fix for an observed diff.

## Non-mismatches — behaviors confirmed identical, and deliberately not "fixed"

- **`helperBad()` is dead code.** The C defines it `static` and never calls it,
  so `bad()` prints only `bad()`. This looks like a bug and was left exactly as
  is; `helper_bad_is_never_called` asserts the string never appears in either
  program's output, so a future "fix" would fail the suite. The Rust keeps the
  function with `#[allow(dead_code)]` to mirror the source.
- **`printLine`'s NULL guard is unreachable.** No input can drive it. It is
  preserved as `Option<&str>` so the structure still matches the C.
- **Write errors do not change the exit status.** With stdout on `/dev/full`
  (ENOSPC) or with fd 1 closed outright (EBADF), the C still exits 0, because
  it discards `printf`'s return value and glibc's exit-time flush failure does
  not alter the status. Verified for both, so the `let _ =` discards in the
  translation are faithful, not sloppy.
- **stdin is never read.** Empty, one line, no trailing newline, 10 000 lines,
  4 KiB of binary including NUL bytes, invalid UTF-8, whitespace only, and fd 0
  closed all produce identical output and status.
- **`argc`/`argv` are ignored.** No args, one arg, an empty-string arg, args
  containing newlines and non-ASCII UTF-8, a 100 000-character arg, 2 000 args,
  a rewritten `argv[0]`, and an empty `argv[0]` all agree.
- **Locale is irrelevant.** `%s` on ASCII literals is locale independent;
  checked under `C`, `C.UTF-8`, `en_US.UTF-8`, `tr_TR.UTF-8` and an invalid
  locale name.
- **stdout on a terminal.** Under a pty (via `script`) both emit the same bytes,
  including the pty's LF→CRLF translation.
- **Determinism.** Five consecutive runs of each are byte-identical.
- **stderr is never written** by either program in any case tested.

## Status

- Both programs build with no errors and no warnings.
- `cargo test`: 29 passed, 0 failed, 0 ignored. No test is disabled, skipped or
  `#[ignore]`d.
- Nothing in `c_src/` was modified. The only addition under it is the generated
  `c_src/build/` CMake output directory, created by the build commands.
