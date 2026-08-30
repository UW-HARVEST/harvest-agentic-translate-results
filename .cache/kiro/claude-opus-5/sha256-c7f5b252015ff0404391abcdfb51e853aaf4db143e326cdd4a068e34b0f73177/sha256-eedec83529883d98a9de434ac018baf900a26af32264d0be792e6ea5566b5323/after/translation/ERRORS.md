# Differential testing report

C ground truth: `c_src/src/main.c` (CWE-482 demonstrator — "unused value").
Rust under test: `translation/src/main.rs`, built as the `driver` binary.

Comparison method: both programs are executed as subprocesses by
`translation/tests/differential.rs`, and **stdout, stderr, exit code and
terminating signal** are compared for each input. The Rust code is never linked
as a library.

## Commands

```sh
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver

# Rust
cd translation && cargo build --release                                 # -> translation/target/release/driver

# Differential suite (builds the C binary itself if it is missing)
cd translation && cargo test
```

## Reference output

Both programs emit exactly these 74 bytes on stdout, nothing on stderr, and
exit 0:

```
Calling good()...
0
2
Finished good()
Calling bad()...
0
0
Finished bad()
```

## Branch enumeration

`main` takes no input: it ignores `argc`/`argv`, never reads stdin, and always
`return 0`. The only conditional in the whole program is the `line != NULL`
guard in `printLine`.

| Location | Branch | Reachable from a program input? | Covered by |
| --- | --- | --- | --- |
| `printLine` | `line != NULL` → print | yes, all four call sites | `no_args_matches` |
| `printLine` | `line == NULL` → print nothing | **no** — every call site passes a string literal; dead code in the executable | n/a (documented, not testable) |
| `printIntLine` | none | — | `c_output_matches_expected_bytes` |
| `good` | `intSum = intOne + intTwo` → prints `0`, `2` | yes | `good_and_bad_sections_are_distinguished` |
| `bad` | `intOne + intTwo;` discarded → prints `0`, `0` | yes | `good_and_bad_sections_are_distinguished` |
| `main` | no conditionals, exit status always 0 | yes | every test |

Because the program has no data input, the remaining input classes are
properties of the process environment rather than of stdin content: argv shape,
stdin content and disposition, stdout writability, environment/locale, and
signal disposition. All are covered in the suite (29 tests).

## Mismatches found and fixed

### 1. `SIGPIPE` disposition — exit status differed (real, deterministic)

**Symptom.** With stdout connected to a pipe that has no reader, the C program
died from signal 13 (`SIGPIPE`) while the Rust program exited 0.

```
C   : signal 13    stderr=b''
Rust: exit 0       stderr=b''
```

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs. A C program launched from a shell inherits `SIG_DFL`. Neither program
checks the return value of its writes (`printf`'s result is discarded in C, and
the translation discards the `writeln!` result to match), so once the signal was
ignored the failed write became invisible and the Rust process ran to a normal
exit 0.

**Fix.** `reset_sigpipe()` in `main` restores `SIG_DFL` for `SIGPIPE` before any
output, putting the Rust process in the same signal state a C process starts in.
Regression test: `stdout_pipe_with_no_reader_matches`, which creates a pipe,
closes the read end *before* spawning the child (so it is deterministic, not a
race) and asserts C dies with signal 13 and Rust does the same.

### 2. stdout buffering granularity — latent divergence (hardened)

**Symptom.** Not observable in the captured byte stream, but the two programs
issued a different number of `write(2)` calls: C stdio fully buffers stdout when
it is not a terminal and flushes once at exit, whereas Rust's `io::stdout()` is
a `LineWriter` that flushes after every newline — eight writes instead of one.

**Why it matters.** After fix 1 made `SIGPIPE` fatal, write granularity becomes
observable in exit status: with a reader that consumes part of the output and
then closes (`prog | head -n 1`), a line-buffered Rust program can get far
enough to be killed by `SIGPIPE` on a later line while the single-write C
program has already handed off all 74 bytes and exited 0.

**Fix.** `COut` emulates C stdio's choice: `isatty(1)` selects a `LineWriter`,
otherwise a `BufWriter` with an 8192-byte capacity (glibc `BUFSIZ`) flushed at
exit. The whole output is 74 bytes, so in the graded (piped/redirected) case it
becomes a single write, exactly as in C.

## Verified as already matching (no fix needed)

- **The `bad()` defect is preserved.** `intOne + intTwo;` is computed and
  discarded, so `intSum` prints `0` twice. The translation keeps this rather
  than "fixing" it to `2`; `good_and_bad_sections_are_distinguished` pins the
  exact eight output lines so a later well-meaning correction fails the suite.
- **`argc`/`argv` are ignored.** Identical output for no args, one arg, an empty
  string arg, flag-like args (`--help`, `-h`, `-v`, `--version`, `-`, `--`,
  `--bad`, `--good`), 256 args, an arg containing a newline, and a 100 000-byte
  arg.
- **Invalid UTF-8 in argv.** Identical behavior; the translation never touches
  `std::env::args()`, which would have panicked where the C is unaffected.
- **stdin is never read.** Identical output for empty stdin, one line, several
  lines, input with no trailing newline, non-numeric text, `int`-overflowing
  numerals, all 256 byte values including NUL, and a 1 MiB payload. Both leave
  the stream fully unconsumed (`neither_program_consumes_stdin`), and both
  behave identically with fd 0 closed.
- **Write failures that do not raise a signal.** With fd 1 closed (`EBADF`) and
  with stdout on `/dev/full` (`ENOSPC`), both programs exit 0 and print nothing
  to stderr, because neither checks the result of a write. A translation using
  `println!` would have panicked with exit 101 and a stderr message here.
- **Environment and locale.** Identical with a cleared environment and under
  `LC_ALL`/`LC_NUMERIC` of `C`, `de_DE.UTF-8`, `en_US.UTF-8` and an invalid
  locale name — `%d` output never picks up a thousands separator.
- **Determinism and stdout type.** 20 consecutive runs produce byte-identical
  output, and redirecting stdout to a regular file (a different libc buffering
  mode) yields the same bytes.

## Status

Both programs build without errors or warnings. `cargo test` passes 29 of 29
tests with none `#[ignore]`d, skipped or disabled. No file under `c_src/` was
modified; the only addition there is the out-of-source `c_src/build/` directory
produced by cmake.
