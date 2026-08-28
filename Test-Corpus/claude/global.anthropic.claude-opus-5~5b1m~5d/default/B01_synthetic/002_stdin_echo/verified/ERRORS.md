# Differential verification of `c_src/src/main.c` against `translation/`

The C program is the ground truth:

```c
int main() {
    char text[128];
    while (fgets(text, 128, stdin)) {
        fputs(text, stdout);
    }
    return 0;
}
```

Both programs are compared by running them:

| program | build | run |
|---|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` | `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` | `translation/target/release/driver` |

`translation/tests/differential.rs` spawns **both binaries as subprocesses** and
compares stdout bytes, stderr bytes and the exit status (a normal exit and death
by signal are rendered differently, so "exited 0" can never match "signalled 13").
It builds the C binary with CMake on first use, so `cargo test` is self-contained.

## Mismatches found and fixed

### 1. `SIGPIPE`: C is killed by signal 13, Rust exited 0

The only mismatch that survived into the initial translation, and an exit-status
one, so a stdout-only comparison would have missed it entirely.

*Reproduction:* pipe a large input through the program and close the read end of
stdout early (`driver < huge | head -c 1`).

*Observed:* C terminated by signal 13 (`SIGPIPE`); Rust exited 0.

*Cause:* the Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs. The failing `write` therefore surfaced as an `EPIPE` error, the loop broke
and `main` returned 0. A C program starts with the default disposition, so the
same `write` kills the process.

*Fix:* `reset_sigpipe()` in `src/main.rs` restores `SIG_DFL` for signal 13 at the
top of `main`, via a direct `extern "C" { fn signal(...) }` declaration (no new
dependency). Covered by `stdout_closed_early_raises_sigpipe`.

### 2. `fgets` discarded a partial line when a read error followed it

*Cause:* the translation returned `None` (i.e. `NULL`) as soon as `fill_buf`
reported an error, throwing away the bytes already consumed. glibc's `_IO_fgets`
returns `NULL` only when the line length is zero *and* the stream is in an error
or EOF state; bytes read before the error are still returned, and the *next*
call is the one that reports `NULL`.

*Fix:* on a read error the refill loop now `break`s instead of returning, so the
function falls through to `if out.is_empty()` and only an empty result becomes
`None`. Reaching this needs a descriptor that fails midway through a line, which
ordinary test input cannot arrange; the surrounding error paths that *are*
reachable are covered by `stdin_that_cannot_be_read` (stdin is a directory,
`EISDIR` on the first read) and `stdin_closed_before_exec` (`EBADF`).

### 3. Output was flushed at line boundaries instead of block boundaries

*Reproduction:* write one short line to stdin, keep stdin open, and try to read
stdout (`short_output_is_withheld_until_eof`).

*Observed:* C, whose stdout is fully buffered on a pipe, wrote nothing until
4 KiB had accumulated or the stream was closed. The translation emitted the line
immediately, because it wrapped `io::stdout()`, and Rust's `Stdout` is *always*
line buffered — there is no way to turn that off through the public API.

*Cause / fix:* `CStdout` now owns descriptor 1 directly (`File::from_raw_fd(1)`
inside a `ManuallyDrop`, so the descriptor is never closed) and applies C's own
rule: line buffered only when stdout is a terminal, otherwise fully buffered,
draining in exact `BUFSIZ` (4096) blocks the way C stdio hands its full buffer to
`write`. The final byte stream was already identical; this aligns *when* the
bytes appear, which is what the "interactive echo" in the C comment depends on.
Covered by `short_output_is_withheld_until_eof` and
`full_blocks_are_flushed_before_exit`.

## C behaviour deliberately reproduced, not "fixed"

- **An embedded NUL truncates the rest of that `fgets` chunk.** `fgets` copies
  NUL bytes into `text` like any other byte, but `fputs` stops at the first one.
  So `printf 'a\0b\n'` prints `a`, and feeding all 256 byte values prints 245 of
  them. The dropped bytes are *not* re-emitted later.
- **Truncation happens once per `fgets` call, so chunking is observable.** A
  600-byte line is read in 127-byte chunks and each chunk is truncated at its own
  first NUL. Changing the chunk size to 128 changes the output — `nul_past_the_first_chunk`
  and `nuls_in_several_chunks_of_one_line` fail if the `size - 1` is dropped.
- **`fgets` never stops at a short read.** It keeps reading until a newline, 127
  bytes, EOF or an error, even if input arrives in dribbles
  (`lines_are_assembled_across_short_reads`).
- **Only `\n` ends a line.** `\r` is ordinary data; CRLF input is copied verbatim.
- **Bytes are bytes.** Invalid UTF-8 passes through untouched — no lossy
  replacement anywhere.
- **Write errors are never reported.** With stdout on `/dev/full` every write
  fails with `ENOSPC`; C ignores the return value of `fputs` and still exits 0
  (`stdout_write_errors_are_not_reported`).
- **Unreadable stdin is not an error either.** `fgets` returns `NULL` on the
  first call and the program exits 0 with empty stdout and empty stderr.
- **Arguments are ignored.** `int main()` declares none, so any `argv` is dropped.
- **stderr is always empty and the exit status is always 0** unless a signal
  kills the process.

## Test inventory

29 tests in `translation/tests/differential.rs`; none is `#[ignore]`d, skipped or
otherwise disabled.

Loop / boundary classes: empty input (loop body never runs), one line, one
character, no trailing newline, blank lines, many lines, lengths 1–257 with and
without a trailing newline (both sides of the 127-byte limit), a newline exactly
after a full buffer, 5000-byte lines, 300 000 bytes with no newline at all,
5000 varied lines.

NUL classes: leading, middle, trailing, several in a row, at indices 0/1/63/125/126
of a full buffer, past the first chunk (126–380), in four separate chunks, a
short NUL line after a full buffer (stale buffer contents), and every byte value.

Byte classes: invalid UTF-8, truncated multibyte sequences, CR/CRLF/lone CR, tab,
ESC, BEL, VT, FF, DEL, plus a deterministic pseudo-random sweep of 40 binary
inputs biased towards `\n` and NUL.

Descriptor classes: stdin is a directory (`EISDIR`), stdin closed (`EBADF`),
stdout closed early (`SIGPIPE`), stdout is `/dev/full` (`ENOSPC`), stdout is a
pipe (full buffering), plus arguments and streaming/short-read timing.

## Confidence check

Each fix was confirmed to be load-bearing by mutating `src/main.rs` and watching
the suite go red, then restoring it:

| mutation | tests that failed |
|---|---|
| drop the `reset_sigpipe()` call | `stdout_closed_early_raises_sigpipe` |
| make `fputs` ignore the NUL terminator | 5 tests, incl. `all_byte_values`, `embedded_nul_truncates_the_line` |
| `let max = size;` (chunk off-by-one) | `nul_at_the_buffer_boundary`, `nul_past_the_first_chunk`, `nuls_in_several_chunks_of_one_line` |
| flush on every `fputs` (line buffering everywhere) | `short_output_is_withheld_until_eof` |

An independent sweep outside the cargo harness (417 inputs: the enumerated edge
cases plus 400 seeded random binary payloads, compared on stdout, stderr and exit
code) reported 0 mismatches. Death by `SIGPIPE` and a pty-backed stdout were also
checked by hand and agree.

## Status

No mismatch remains. `cargo test` passes in both the debug and release profiles,
both programs build without errors, and nothing in `c_src/` was modified.
