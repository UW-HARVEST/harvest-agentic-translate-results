# Differential testing notes: C (`c_src`) vs Rust (`translation`)

Both programs are compared by running them: `c_src/build/driver` (built with
`cmake .. && cmake --build .`) and `translation/target/{debug,release}/driver`.
`tests/differential.rs` feeds identical stdin to both and compares stdout,
stderr and exit status (including the terminating signal) byte for byte.

## What the C program does

```c
int foo(const char *in, char c) {         /* count occurrences of c */
    int res = 0;
    for (const char *s = in; s = strchr(s, c); s++) res++;
    return res;
}
void driver(const char *in) { printf("A: %d\n", foo(in,'A')); printf("x: %d\n", foo(in,'x')); }
int main() { char in[1000] = ""; fread(in, 1, sizeof(in), stdin); driver(in); return 0; }
```

Input classes that follow from this: no input, one byte, matches vs near
misses ('a', 'X'), newlines (irrelevant: `fread`, not `fgets`), an embedded NUL
(the buffer is zero-initialised, so the first NUL ends the string `foo` walks),
bytes above 0x7F, exactly 999 bytes, exactly 1000 bytes, more than 1000 bytes
(the excess is never read), stdin arriving in several short reads, stdin closed
or unreadable, and stdout that cannot be written.

## Mismatches found and fixed

### 1. SIGPIPE: Rust exited 0 where the C was killed by signal 13

Observed with stdout connected to a pipe whose reader had already exited:

| | exit status |
|---|---|
| C | killed by SIGPIPE (signal 13) |
| Rust (before) | exited 0, no stderr |
| Rust (after) | killed by SIGPIPE (signal 13) |

Cause: the Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs, so the failing `write` returned `EPIPE` (which this program ignores, like
the C ignores `printf`'s return value) instead of killing the process. A C
program starts with the default disposition, so `printf`/`fflush` to a dead
pipe terminates it.

Fix: `restore_default_sigpipe()` in `src/main.rs` calls `signal(SIGPIPE,
SIG_DFL)` as the first statement of `main`. stdout and stderr bytes were
already identical; only the exit status differed, which is exactly the failure
mode that a stdout-only comparison would have missed.

Covered by `stdout_is_a_pipe_with_no_reader`.

### 2. Input that exactly fills the buffer reads out of bounds in the C

First seen with a 1200-byte file on stdin (first 1000 bytes = 700 'A' then
300 'x'), then reproduced repeatedly with a 1000-byte input of all 'A':

| | stdout |
|---|---|
| C, most runs | `A: 1000\nx: 0\n` |
| C, ~2-3% of runs | `A: 1001\nx: 0\n` |
| Rust | `A: 1000\nx: 0\n` |

Cause: undefined behaviour in the C, not a translation defect.
`fread(in, 1, sizeof(in), stdin)` can fill all 1000 bytes of `in`, consuming the
slot that held the zero initialiser, so the buffer is no longer NUL-terminated
and `strchr` walks past its end into whatever the stack holds after it. When
that byte is 0 (the usual case, and what glibc's zero-low-byte stack canary
gives you) the count stops at the buffer edge; when it happens to be `0x41`
('A') or `0x78` ('x') the count comes out one too high.

Measured behaviour:

- Run from a shell, ~6000 invocations across file-backed and pipe-backed stdin
  at both `-O0` and `-O3`: never deviated.
- Run from `cargo test` (a much larger environment, which shifts the stack and
  leaves a live pointer rather than a zero byte just past the buffer): deviates
  in roughly 2-3% of runs, always by counting one extra match, never fewer.
  `setarch -R` (ASLR off) did not deviate, consistent with the stray byte being
  part of an address that varies per run.

Resolution: the Rust implements the well-defined interpretation, which is also
what the C does whenever that out-of-bounds byte is 0 - the string ends at the
first NUL byte **or** at the end of the 1000-byte buffer, whichever comes first.
No Rust change can reproduce arbitrary stack residue from the C process.

Because the C is not a deterministic function of its input for this class, the
tests that cover it call `assert_same_full_buffer`, which runs the C nine times,
takes the output it produces in the majority of runs as the reference, and
requires the Rust output to equal it byte for byte. Deviating C runs are not
ignored: they must keep the exact `A: %d\nx: %d\n` shape and report counts no
*lower* than the reference, which is the only effect an out-of-bounds read past
the end of the input can have. A C run that reported fewer matches, a different
format, different stderr or a different exit status fails the test. Deviations
that do occur are logged (visible with `cargo test -- --nocapture`).

Tests using this reference: `length_1000_fills_the_buffer`,
`length_1000_mixed`, `length_1001_drops_the_last_byte`,
`longer_than_buffer_is_truncated`, `much_longer_than_buffer`,
`matches_only_beyond_the_buffer_are_invisible`,
`stdin_delivered_in_small_slow_chunks`, `stdin_is_a_regular_file` and the
`n >= 1000` cases of `length_sweep_around_the_boundary`. Every other input -
including 999 bytes, and 1000 bytes whose last byte is a NUL
(`length_1000_ending_in_nul_is_terminated`) - is compared strictly, with a
single run of each program.

## Behaviours checked and already matching

- Empty stdin, `/dev/null`, an empty regular file, and stdin closed outright
  (`fread` fails, the zeroed buffer is used, output `A: 0\nx: 0\n`, exit 0).
- Newlines, CRLF and whitespace are ordinary bytes: `fread` does not stop at
  them.
- Case sensitivity: 'a' and 'X' are not counted.
- Adjacent matches: the `s++` after a hit still counts runs like `AAAA`.
- A NUL anywhere in the first 1000 bytes hides everything after it.
- Bytes 0x80..0xFF and invalid UTF-8 pass through without affecting the counts
  (the Rust works on `[u8]`, never `str`).
- Input longer than 1000 bytes: the remainder is never read, and unread
  matches are not counted.
- stdin delivered in 25-byte chunks with pauses: both keep reading until the
  buffer is full or the stream ends (a single `read` per program would have
  under-counted).
- stdout that always fails (`/dev/full`): both ignore the write error, print
  nothing to stderr and exit 0.
- Command-line arguments are ignored by both.
- Output format is exactly `A: %d\nx: %d\n`, both lines newline-terminated,
  stderr always empty, exit code always 0 (absent a signal).
