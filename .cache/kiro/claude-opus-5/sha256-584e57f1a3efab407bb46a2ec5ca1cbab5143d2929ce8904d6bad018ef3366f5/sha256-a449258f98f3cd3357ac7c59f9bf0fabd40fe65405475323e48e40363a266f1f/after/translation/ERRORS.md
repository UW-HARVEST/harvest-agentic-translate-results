# ERRORS.md — mismatches found while verifying the C→Rust translation

Ground truth (`c_src/src/main.c`, verbatim body):

```c
int main() {
    printf("Hello World!\n");
    return 0;
}
```

C binary: `c_src/build/driver` (cmake + gcc 11.5.0)
Rust binary: `translation/target/release/driver` (rustc 1.97.1)

## Enumerated input classes

`main` takes no parameters and the body reads nothing, so there is no parsing,
no length limit and no `if`/early-`return` error path in the C source. There is
exactly one code path. Every remaining degree of freedom is environmental, and
each was tested (see `tests/differential.rs`):

| Class | Cases |
|---|---|
| stdin contents (never read by C) | empty, bare `\n`, one item with and without trailing newline, whitespace only, multiple lines, non-numeric garbage, numeric extremes incl. `INT_MAX`, `INT_MAX+1`, `INT_MIN-1`, `2^32`, `2^63`, 26-digit overflow, invalid UTF-8 / NUL bytes, 1 MiB payload, 10 000 lines |
| stdin type | pipe with data, pipe at EOF, `/dev/null` |
| argv (ignored by `int main()`) | none, one, `--help`, `-h`, `--version`, empty string, several, negative/overflow-looking, args containing spaces and tabs, 200 args |
| environment | `LC_ALL` = C / en_US.UTF-8 / de_DE.UTF-8 / tr_TR.UTF-8, `LC_NUMERIC` = de_DE.UTF-8 |
| stdout target | pipe, regular file (block buffered), merged with stderr into one file, unwritable fd, pipe whose reader is closed |
| repeatability | 25 sequential runs, 8 concurrent runs |

## Mismatch 1 — broken stdout pipe: Rust exited 0 where C is killed by SIGPIPE

**Status: found and fixed.**

Reproduction (read end of the stdout pipe closed before the child writes):

```python
import os, subprocess
for p in ["c_src/build/driver", "translation/target/release/driver"]:
    r, w = os.pipe(); os.close(r)
    pr = subprocess.Popen([p], stdout=w, stderr=subprocess.PIPE); os.close(w)
    pr.communicate()
    print(p, pr.returncode)
```

Observed before the fix:

| | stdout | stderr | status |
|---|---|---|---|
| C | (empty) | (empty) | killed by signal 13 (`SIGPIPE`) → shell status 141 |
| Rust | (empty) | (empty) | exit code 0 |

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` in its
pre-`main` runtime initialization. A C program started from a shell inherits
`SIG_DFL`, so `printf`'s write to a pipe with no reader raises `SIGPIPE` and the
process dies. In the Rust build the signal was ignored, `write_all` returned
`EPIPE`, the error was discarded (matching C's habit of not checking `printf`),
and `main` returned 0 — the same stdout and stderr as C, but a different exit
status. A stdout-only comparison would not have caught this; it is exactly the
"exits 0 where the C exits non-zero" failure mode.

**Fix.** `translation/src/main.rs` now restores the default disposition as the
first statement of `main`, before any output is produced:

```rust
const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;
unsafe extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
unsafe { signal(SIGPIPE, SIG_DFL); }
```

After the fix both programs are killed by signal 13 with empty stdout and
stderr. Covered by `stdout_pipe_with_closed_reader_sigpipe`. That test was
confirmed to be non-vacuous: commenting out the `restore_default_sigpipe()`
call makes it fail with `status differs on broken stdout pipe`, and it passes
again once restored.

## Checks that matched with no change required

- Output bytes are exactly `48 65 6c 6c 6f 20 57 6f 72 6c 64 21 0a`
  (`Hello World!\n`): one trailing `\n`, no `\r`, no extra newline from Rust's
  `write_all` (a `println!`-style translation would have been equally correct
  here, but the byte comparison pins it either way).
- stderr is empty for both in every case.
- Exit status 0 for every non-signal case.
- stdin is never consumed by either program, including the 1 MiB and
  10 000-line payloads; neither blocks nor reports an error, and the harness
  tolerates `EPIPE` when writing to a child that has already exited.
- Unwritable stdout (`>&-` equivalent, a read-only fd as fd 1): the write fails,
  neither program checks it, both exit 0 with empty stderr.
- Locale has no effect: the format string contains no numeric conversions or
  non-ASCII text.
- Buffering mode (line-buffered pipe vs block-buffered file) produces the same
  bytes; there is only one write and it is flushed at exit in both programs.

## Verification commands

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                  # -> translation/target/release/driver
cd translation && cargo test                                             # 23 differential tests, 0 ignored
```
