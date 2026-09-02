# Differential findings: `c_src/src/main.c` vs `translation/src/main.rs`

Method: build both executables, run them as subprocesses under identical argv,
stdin, environment and descriptor setups, and compare stdout, stderr and exit
status (including terminating signal). Nothing under `c_src/` was modified; the
C program is the ground truth and every fix below was made to the Rust side.

- C reference: `c_src/build/driver`, built with
  `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
- Rust: `translation/target/release/driver`, built with
  `cd translation && cargo build --release`
- Tests: `cd translation && cargo test` (30 tests, none ignored or skipped)

## Input classes enumerated from the C source

`main` accepts `argc`/`argv` but branches on neither, and the program never
reads stdin (no `scanf`, no `fgets`, no `getchar`). The only conditional in the
whole translation unit is the `line != NULL` guard inside `printLine`, and every
call site passes a string literal, so the NULL arm is unreachable. `helperBad`
is `static` and never referenced. The transcript is therefore a fixed 92 bytes
on every input:

```
Calling good()...
good()
helperGood()
Finished good()
Calling bad()...
bad()
Finished bad()
```

Because the *inputs* do not branch, the branches that do exist live in the
runtime environment, so that is where the test suite concentrates: argv shape,
stdin shape, locale, and the state of the stdout descriptor.

| Class | Cases covered |
| --- | --- |
| argv | none, one, empty string, flag-like (`-h`, `--`, `-`, …), non-UTF-8 bytes, 64 KiB argument, 2000 arguments |
| stdin | empty, one line, no trailing newline, whitespace/numeric soup, all 256 byte values, 1 MiB unread, descriptor closed |
| environment | `LC_ALL=C`, `en_US.UTF-8`, `tr_TR.UTF-8`, `LANG=de_DE.UTF-8`, `LC_NUMERIC=de_DE.UTF-8` |
| stdout | pipe drained, pipe with a dead reader, regular file, `/dev/null`, `/dev/full` (every write fails ENOSPC), descriptor closed (EBADF), pseudo-terminal, merged with stderr |

## Mismatch 1 — exit status on a broken stdout pipe

**Symptom.** With stdout on a pipe whose read end was already closed, the C
program exited with status 141 while the Rust program exited 0.

```
phase_c_stdout_is_a_pipe_whose_reader_is_already_gone
  left:  (Some(141), None)     # C
  right: (Some(0),   None)     # Rust
```

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` during runtime
startup, before `main` is entered. A C program inherits the default disposition.
So a failing write made the C program die from signal 13 — shell status 141 —
whereas in Rust the write merely returned `EPIPE`, which the translation
discarded (matching C's habit of ignoring `printf`'s return value), after which
`main` fell through to `exit(0)`.

**Fix.** `translation/src/main.rs` now restores the C starting state with
`signal(SIGPIPE, SIG_DFL)` as the first statement of `main`, before any output
is produced. `signal` and `isatty` are declared in a small `extern "C"` block so
the crate keeps its dependency-free `Cargo.toml`.

Verified out of band as well as in the suite: with the read end of a pipe closed
before launch, both binaries now report `returncode=-13`, i.e. killed by
`SIGPIPE`.

## Mismatch 2 — stdout buffering discipline

**Symptom.** Not a difference in bytes, but a difference in *when* those bytes
reach the descriptor, which changed the outcome of the broken-pipe race. Running
`prog | true` twenty times, the failures were one-directional: C reported 141
while Rust reported 0, never the reverse.

**Cause.** The original translation wrote through `std::io::stdout()`, which is
a `LineWriter` regardless of what stdout is connected to. That produced seven
separate writes, the first of them almost immediately after startup. C stdio
instead picks its mode from the descriptor: line buffered when `isatty(1)`,
fully buffered otherwise. Against a pipe, the C program accumulated all 92
bytes and emitted them in a single write during exit-time flush. Rust was
therefore writing early and often where C wrote once and late, giving the two
programs systematically different exposure to a pipe that was in the process of
closing.

**Fix.** `print_line` now appends into a process-wide buffer and flushes it the
way C stdio would — immediately when `isatty(1)`, otherwise only once 4096 bytes
have accumulated (glibc sizes a fully buffered stream from the descriptor's
`st_blksize`) or at exit. The flush writes straight to descriptor 1 through a
`ManuallyDrop<File>` so it lands in one `write` call, and write errors are
discarded exactly as the C code discards `printf`'s return value.

**Result.** The remaining `prog | true` divergences became symmetric — both
`c=141 r=0` and `c=0 r=141` now occur, at roughly equal rates — which is the
signature of an inherent scheduling race between two separately launched
processes rather than a behavioural difference. The deterministic form of the
same scenario, where the reader is confirmed dead before launch, agrees on every
run and is what the test suite asserts. The racy shell pipeline is deliberately
not asserted on.

## Harness defect found along the way (not a translation defect)

Worth recording because it produced a failure that looked like a translation
mismatch. `phase_c_stdout_is_a_pipe_whose_reader_is_already_gone` began failing
only when the whole suite ran, and passed every time in isolation.

The cause was in the test file, not in either program. The pty test obtained the
terminal slave with `Stdio::from_raw_fd(slave_fd)` — which transfers ownership,
so `spawn` closes the parent's copy — and then called `close(slave_fd)` by hand
as well. That second close targets a descriptor *number* that the runtime is
free to have reassigned, and because `cargo test` runs tests on parallel threads
it sometimes landed on the write end of the pipe belonging to the broken-pipe
test, which then saw a healthy pipe instead of a broken one.

Fixed by passing `Stdio::from(File)` and letting `spawn` close the parent's
descriptor, with no manual `close`. Ten consecutive full runs of the suite now
pass, 30 tests each.

The lesson generalises: a test that manipulates raw descriptors has to respect
single ownership, or it will report mismatches that the program under test never
committed.

## Checks that found no mismatch

Recorded so the next reader knows these were actually exercised, not assumed.

- Exact transcript: seven lines, one `\n` each, no trailing blank line, no `\r`,
  no leading or trailing spaces — `printf("%s\n", line)` with no field width.
- `helperBad()` never appears on stdout, and `helperGood()` appears exactly
  once. A translation that "tidied up" `bad()` to call its unused helper, by
  symmetry with `good()`, would be caught here.
- stderr is empty in every case, and the exit status is 0 in every case that is
  not a broken pipe.
- argv is never inspected: 2000 arguments, a 64 KiB argument, an argument that
  is not valid UTF-8, and an empty-string argument all yield the same bytes. The
  non-UTF-8 case matters because a `char *` carries no encoding guarantee, so a
  translation that eagerly converted `args()` to `String` would panic.
- stdin is never read: 1 MiB of unread input does not deadlock or truncate
  either program, and a closed descriptor 0 changes nothing.
- Locale does not reach `printf` here — no `setlocale` call, and no numeric
  conversions in the format string.
- Failing writes are ignored identically. Against `/dev/full` every write fails
  with `ENOSPC`, and against a closed descriptor 1 with `EBADF`; both programs
  stay silent on stderr and exit 0.
- On a pseudo-terminal both produce the same 99 bytes, the terminal driver's
  `ONLCR` having expanded each `\n` to `\r\n` for both equally. This is the case
  that exercises the line-buffered branch of the buffering fix.
- Output is byte-identical across repeated runs; nothing depends on a pointer
  value, a hash seed or a timestamp.

## Unreachable in the C source, and left unreachable

`printLine(NULL)` and `helperBad()` cannot be triggered through any input, so no
test drives them. The Rust translation keeps both shapes — `Option<&str>` with
its `None` arm, and `helper_bad` behind `#[allow(dead_code)]` — so the structure
still corresponds to the C, but neither can affect observable behaviour.
