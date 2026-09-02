# Differential verification: C vs. Rust `driver`

The C original (`c_src/src/main.c`) is the ground truth:

```c
#include <stdio.h>

/* interactive echo; ignores arguments, copies stdin to stdout */
int main() {
    char text[128];

    while (fgets(text, 128, stdin)) {
        fputs(text, stdout);
    }
    return 0;
}
```

## How it was verified

Both programs are built and run as subprocesses; stdout, stderr and exit status
(plus termination signal on Unix) are compared byte for byte.

- C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  produces `c_src/build/driver`.
- Rust: `cd translation && cargo build --release` produces
  `translation/target/release/driver`.
- Tests: `cd translation && cargo test` — 43 tests in `tests/differential.rs`,
  none `#[ignore]`d, none skipped.

The Rust code is never loaded as a library; the built binary is invoked the way a
shell would invoke it.

## Mismatches found and fixed

### 1. stdout was line-buffered instead of block-buffered

**Severity: real, observable.** This was the only behavioral divergence found.

The first translation wrote through `io::stdout()`, which is a `LineWriter`: it
flushes on every `\n`. C stdio picks the buffering mode from the descriptor —
line-buffered only when stdout is a terminal, otherwise fully buffered in
`st_blksize`-sized blocks (4096 here; `BUFSIZ` is 8192 but glibc prefers
`st_blksize`).

Two distinct observable consequences:

*Output lost when the process is killed by a signal.* With stdin on a fifo,
stdout on a file, and a `SIGTERM` arriving after `hello\nworld\n` was echoed:

| | exit status | bytes on stdout |
|---|---|---|
| C | 143 (SIGTERM) | 0 — the 12 bytes died in the stdio buffer |
| Rust (before) | 143 (SIGTERM) | 12 — `LineWriter` had already flushed |

*Flush boundaries visible to a concurrent reader.* Feeding 5000 bytes of
complete lines and inspecting stdout while the program is still running: C has
written exactly 4096 bytes (one full block), the old Rust had written all 5000.

**Cause.** `io::stdout()`'s buffering policy is a Rust convention, not C's. The
translation adopted Rust's policy rather than reproducing C stdio's.

**Fix.** `src/main.rs` now implements a `CStdout` that mirrors C stdio:
`isatty(1)` selects line buffering (flush through the last newline), otherwise it
block-buffers with capacity taken from `fstat(1).st_blksize` (falling back to
`BUFSIZ` = 8192 when that is 0 or unavailable). Crucially it fills to an *exact*
buffer boundary and flushes the full buffer, then continues with the remainder.
Rust's `BufWriter` does not do this — it flushes whatever partial content it
holds when the next write would not fit, so it would emit e.g. 4050-byte chunks
where C emits 4096-byte chunks.

Both now report 4096 bytes visible mid-run and 0 bytes surviving `SIGTERM`.
Covered by `flush_boundaries_match` and
`buffered_output_lost_on_signal_identically`.

### 2. A failed write aborted the echo loop

**Severity: latent — no output difference observed, but the C semantics differ.**

C discards `fputs`'s return value, so a write failure does not end the loop: the
program keeps calling `fgets` and draining stdin, and still returns 0. The first
translation did `if writer.write_all(..).is_err() { break; }`.

Exit status matched (0 either way) and neither program produces visible output on
a dead stdout, so no test caught this directly. It is still a divergence: the
amount consumed from stdin differs, which is observable when stdin is a pipe
shared with another reader.

**Fix.** `CStdout::write_out` swallows write errors and the loop runs to EOF,
matching the discarded `fputs` return. `unwritable_stdout_still_exits_zero`
exercises the path via `/dev/full`.

### 3. stdin refill granularity

**Severity: latent, defensive.** `StdinLock` uses a fixed 8 KiB `BufReader`;
glibc refills in `st_blksize` units (4096 for pipes and files here). As with #2
this is only observable if the process dies mid-stream while another process
reads the same pipe. `CStdin` now takes its capacity from `fstat(0).st_blksize`
so the consumption pattern matches.

## Behaviors that were already correct and are now pinned by tests

These are the parts of the C that look like bugs but are the specification:

- **`fgets` reads at most 127 bytes**, not a whole line. `char text[128]` with
  `fgets(text, 128, ...)` means a 200-byte line is echoed as a 127-byte chunk
  then a 73-byte chunk. The concatenation is unchanged, so this is only visible
  through flush timing — but the 127-byte limit must be exact, because it decides
  where an embedded NUL lands. Covered by
  `lengths_around_buffer_boundary_with_newline` (0, 1, 125, 126, 127, 128, 129,
  253–257), `exactly_127_bytes_then_newline_then_more`, and
  `nul_exactly_at_last_buffer_slot`.

- **A 127-byte line plus its newline takes two `fgets` calls.** The first fills
  the buffer without seeing `\n`; the newline is returned by the next call.

- **An embedded NUL silently truncates the chunk.** `fgets` reads past NUL bytes,
  but `fputs` writes a C string and stops at the first one. Input `abc\0def\n`
  echoes `abc` — the `def\n` is discarded, not echoed later. Input `\0\n` echoes
  nothing at all. This is data loss and it is correct behavior. Covered by
  `embedded_nul_truncates_output`, `leading_nul_suppresses_whole_line`,
  `nul_only`, `nul_then_newline`, `nul_at_end_of_line`, `many_nuls`,
  `nul_past_buffer_boundary`, `nul_exactly_at_last_buffer_slot`.

- **`fgets` returns NULL only when it read nothing.** glibc's `fgets` returns the
  buffer whenever `count > 0`, even if a read error followed, and returns NULL
  only for `count == 0 && ferror`. So a partial final line at EOF *is* echoed.
  Covered by the `*_without_newline` / `*_unterminated` cases.

- **No trailing newline is added.** Input `hello` produces exactly `hello`.

- **argv is ignored**, including things that look like flags (`--help`,
  `--version`). Covered by `arguments_are_ignored`.

- **Exit status is always 0**, including empty stdin, closed stdin, stdin
  pointing at a directory (`EISDIR`), and unwritable stdout. Covered by
  `empty_input`, `stdin_closed`, `stdin_from_dev_null`,
  `unwritable_stdout_still_exits_zero`.

- **No UTF-8 validation.** Arbitrary bytes including `0xFF`, lone continuation
  bytes, and multi-byte characters split across the 127-byte boundary pass
  through untouched. A `String`-based translation would corrupt these. Covered by
  `invalid_utf8_bytes`, `truncated_utf8_sequence_across_buffer_boundary`,
  `all_byte_values_one_per_line`, `all_byte_values_one_chunk`,
  `binary_blob_no_newlines`.

- **Only `\n` ends a line.** `\r`, `\v`, `\f` do not. Covered by
  `crlf_line_endings`, `lone_carriage_returns`,
  `vertical_tab_and_form_feed_are_not_line_breaks`.

- **SIGPIPE keeps its default disposition**, so `driver` writing into a closed
  pipe dies with signal 13 (status 141 through a shell). Rust's runtime masks
  SIGPIPE at startup, which would have made the Rust program exit 0 where the C
  program is killed. `restore_default_sigpipe()` reinstates `SIG_DFL`; verified
  manually — both report 141.

## Additional verification beyond the test suite

- 400 randomized inputs (seeded, lengths 0–3000, alphabets biased toward `\n`,
  NUL and high bytes): 0 mismatches across stdout, stderr and exit status.
- 12 MB / 60 000-line stream through a file-backed stdin: byte-identical output,
  both exit 0.
- Under a pty (`script -q`): identical output, confirming both line-buffer when
  stdout is a terminal.
- `c_src/` sources unmodified — `c_src/src/main.c` md5 `4975497de39fd545f46d1dd
  ffc6d5d07`, `c_src/CMakeLists.txt` md5 `4fdd0b4c6c59a1168f1e689a19c52f28`,
  both with their original mtimes. Only `c_src/build/` was added, by the
  documented cmake invocation.

## Known limits of this verification

Terminal-interactive behavior is only checked through `script`; a real
interactive session (partial reads with no EOF, `\x04` handling by the tty
driver) is not exercised. The buffer size is read from `st_blksize` at runtime
rather than hardcoded, so this tracks glibc on other filesystems, but a C library
that chooses buffer sizes differently from glibc would diverge on flush
boundaries.
