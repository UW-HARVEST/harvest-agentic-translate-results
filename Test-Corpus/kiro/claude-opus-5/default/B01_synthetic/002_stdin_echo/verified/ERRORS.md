# Mismatches found while verifying the translation

The C program (`c_src/src/main.c`) is a 128-byte `fgets`/`fputs` echo loop and is
the ground truth. Each entry below is a behavioral difference that differential
testing exposed (or that reading glibc's source showed the Rust code would get
wrong), together with the cause and the fix applied to `translation/src/main.rs`.
The C sources were not modified.

## 1. Exit status on a closed stdout: C died from SIGPIPE, Rust exited 0

**Symptom.** With the reader of stdout going away mid-stream, the two programs
disagreed on how they terminated:

```
$ ./c_src/build/driver < big.bin | head -c 10 > /dev/null ; echo ${PIPESTATUS[0]}
141                     # killed by SIGPIPE (128 + 13)
$ ./translation/target/release/driver < big.bin | head -c 10 > /dev/null ; echo ${PIPESTATUS[0]}
0                       # exited normally
```

stdout bytes matched; only the exit status differed, which is exactly the class
of mismatch a stdout-only assertion would have missed.

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` during runtime
startup, before `main` runs. Writing to a closed pipe therefore surfaces as an
ignored `EPIPE` error (the C code ignores `fputs`'s return value, so the Rust
translation ignored the write result too) and the process ran to a normal exit.
A C program inherits the default disposition and is killed by signal 13.

**Fix.** Restore `SIG_DFL` for `SIGPIPE` as the first statement of `main`
(`restore_default_sigpipe`, declaring `signal(2)` via `extern "C"` so no new
dependency is needed). Covered by `stdout_closed_early_kills_both`.

## 2. Partial line kept after a read error, where glibc discards it

**Symptom.** No input reproduced this with an ordinary file or pipe stdin, so it
was found by reading glibc rather than by a failing test. The stdin-is-a-
directory test (`EISDIR` on the first `read`) passes either way, because no
bytes have been stored when the error arrives.

**Cause.** The Rust `fgets` treated a read error like end-of-input: it stopped
filling the buffer and still returned the bytes gathered so far. glibc's
`fgets` checks the stream's error indicator *after* reading and returns NULL
whenever it is set and `errno != EINTR`, even if some bytes were already stored
— that partial line is never passed to `fputs`. Because the error indicator is
sticky, the C `while` loop also terminates at that point.

**Fix.** Return NULL (`false`) immediately from `fgets` on a non-`EINTR` read
error instead of returning the partial buffer. `ErrorKind::Interrupted` still
retries, matching glibc's `errno != EINTR` carve-out.

## 3. stdout buffer size mismatch (fidelity, not a diff on its own)

**Cause.** `BufWriter::new` uses an 8 KiB buffer, whereas glibc gives a piped
`stdout` a fully buffered `FILE` sized from the pipe's block size (4096 on
Linux). This cannot change the bytes of a completed run, but it changes how much
output has escaped when a fatal `SIGPIPE` arrives (issue 1), i.e. the partial
output of an interrupted run.

**Fix.** Build the writer with `BufWriter::with_capacity(4096, ...)`.

---

## C behavior deliberately preserved (verified, not "fixed")

These look like bugs and are reproduced exactly:

- **A NUL byte truncates the rest of the chunk.** `fgets` happily stores NUL
  bytes, but `fputs` writes only up to the first one. So `ab\0cd\nef\n` echoes
  as `abef\n` — `cd\n` is silently dropped. Input consisting only of `\0` or
  `\0\n` produces no output at all yet still exits 0.
  (`nul_truncates_the_chunk`, `nul_interacts_with_the_buffer_boundary`)
- **Lines are split at 127 bytes.** `fgets(text, 128, stdin)` stores at most
  `128 - 1` bytes, so a longer line is echoed in 127-byte pieces. The
  reassembled output is identical, but the NUL truncation above applies per
  piece, so a NUL's blast radius ends at the next 127-byte boundary.
  (`line_length_at_buffer_boundary`, `newline_exactly_after_a_full_buffer`)
- **A final line with no trailing newline is echoed as-is**; `fgets` returns
  non-NULL for it and no newline is added.
- **`\r` is not a line terminator.** Only `\n` ends a `fgets` chunk; CRLF and
  lone-CR input pass through untouched.
- **Arbitrary bytes pass through**, including invalid UTF-8 — the loop is
  byte-oriented, so nothing is validated or replaced.
- **All arguments are ignored.** `main()` is declared without parameters, so
  `-h`, `--version` and file names change nothing; input still comes from stdin.
- **Read errors are silent.** With a directory as stdin, `fgets` returns NULL on
  the first call: no output, nothing on stderr, exit 0.
- **Write errors are silent too.** The return value of `fputs` is unchecked, so
  a failed write neither stops the loop nor changes the exit status (absent a
  signal). Nothing is ever written to stderr, on any path.

## How to reproduce

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
cd translation && cargo test                                            # differential suite (builds the C program if needed)
```

`translation/tests/differential.rs` runs both binaries as subprocesses with
stdin redirected from a file and compares stdout, stderr and the exit status
(including death by signal) for every input class above. No test is ignored or
skipped.
