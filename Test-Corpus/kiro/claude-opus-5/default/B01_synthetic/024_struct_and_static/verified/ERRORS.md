# Differential verification log — `c_src/src/main.c` vs `translation/`

The C program is ground truth. Both executables are built and run as
subprocesses; stdout, stderr, exit code and terminating signal are compared for
every input class. This file records each mismatch that was found and what
caused it.

## How to reproduce

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
cd translation && cargo build --release && cargo test
```

`translation/tests/common/mod.rs` builds the C program automatically on first
use, so `cargo test` alone is sufficient.

## Branch inventory of the C program

`main` is straight-line code: `scanf("%d", &x)`, `run(x)` twice, `return 0`.
There is no `if` in the program, so all input-dependent behavior comes from
`scanf`, from signed `int` arithmetic in `add_bedrooms`, and from the process's
output channel:

| Class | Input example | C behavior |
| --- | --- | --- |
| EOF before any conversion | `""` | no assignment, `x` stays `0` |
| whitespace only | `" \t\n\v\f\r"` | `%d` skips it, then hits EOF; `x` stays `0` |
| leading whitespace then digits | `"  \n 7\n"` | `%d` reads across newlines |
| non-numeric first byte | `"abc"`, `".5"`, `"\0"` | matching failure, `x` stays `0` |
| sign not followed by a digit | `"-"`, `"+ 3"`, `"--5"` | matching failure, `x` stays `0` |
| digits | `"5"`, `"-3"`, `"+4"`, `"007"` | converted |
| digits then anything | `"3 4"`, `"12abc"`, `"0x10"`, `"1.9"` | only the leading digits are consumed; the rest is never read |
| fits `long`, not `int` | `"2147483648"`, `"4294967297"` | glibc truncates the `long` to `int` |
| overflows `long` | `"9"*10000`, `"-9"*10000` | glibc saturates to `LONG_MAX`/`LONG_MIN`, then truncates |
| `bedrooms` addition overflows | `"2147483647"`, `"-1073741824"` | signed overflow; wraps in the compiled binary |
| argv | any arguments | `main()` takes no parameters, argv ignored |
| exit status | any input | always `0` |
| stdout write fails with `EPIPE` | reader closed | process killed by `SIGPIPE` (13) |
| stdout write fails otherwise | fd 1 closed, `/dev/full` | return value ignored, exit `0` |

## Mismatches found

### 1. `SIGPIPE`: Rust exited 0 where C died by signal 13

*Found in Phase C, while enumerating output-channel behavior rather than input
behavior.*

Reproduction — stdout is a pipe whose read end is closed before the program
writes:

```
c_src/build/driver                  returncode=-13  stderr=b''
translation/target/release/driver   returncode=0    stderr=b''
```

Cause: two things compounding.

1. The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main` runs.
   A C program inherits the default disposition, so a `printf` flush to a
   closed pipe terminates it with signal 13. With the signal ignored, the Rust
   write instead returned `Err(EPIPE)`.
2. `print_the_house` discarded the write result with `let _ = write!(...)`,
   which is correct for mirroring C's ignored `printf` return value, but it
   meant the `EPIPE` was swallowed and `main` returned normally with status 0.

Fix (`translation/src/main.rs`): `restore_default_sigpipe()` is called first in
`main` and resets `SIGPIPE` to `SIG_DFL` via the libc `signal` symbol, so the
failing write kills the process exactly as it kills the C program. The write
result is still ignored, which is what keeps the `/dev/full` and closed-`fd 1`
cases at exit 0.

Regression test: `tests/output_channels.rs::write_to_closed_pipe_matches`.

### 2. stdout buffering granularity (latent, fixed alongside #1)

Not observed as a byte difference, but fixed because it determines *when* the
`SIGPIPE` in #1 can be raised.

C's `stdout` is block-buffered when it is not a terminal. This program emits 432
bytes, well under glibc's 4096-byte buffer, so all eight lines leave the process
in a single `write` at exit. The Rust translation wrote through
`std::io::stdout()`, a `LineWriter`, producing eight separate writes.

Fix: output is accumulated in a `STDOUT_BUF` thread-local and written once by
`flush_stdout()` at the end of `main`, mirroring the implicit `fflush(stdout)`
that `exit` performs.

## Checks that passed with no mismatch

Recorded so a later reader knows they were tried rather than skipped.

- `%.1f` formatting. The bathroom count only ever takes the values 2.5, 3.5 and
  4.5, all exactly representable, so no rounding-mode difference between
  glibc's `printf` and Rust's `{:.1}` is reachable.
- Locale. The C program never calls `setlocale`, so it stays in the "C" locale
  and `%.1f` keeps a `.` separator even under `LC_ALL=de_DE.UTF-8`. Rust is
  locale-independent, so the two agree. Covered by
  `locale_does_not_change_formatting`.
- `long`-overflow saturation, including 10 000-digit inputs and inputs padded
  with 100 leading zeros.
- Truncation of `long` to `int`, including `2147483648`, where the two
  `bedrooms` additions wrap by 2^31 each and cancel, leaving the final count at
  the initial 5.
- Every single byte `0x00..=0xff` as the sole input, and every byte
  `0x00..=0xff` following a digit, exercising the whitespace / sign / digit /
  other partition of `scanf`'s scan exhaustively.
- 1 MB of leading spaces before the digit.
- Closed stdin (`fd 0` unreadable) and empty stdin.
- argv ignored: extra, empty and `--help`-style arguments.
- stdout redirected to a regular file.
- 12 000 randomized inputs drawn from an alphabet of whitespace, signs, digits,
  `.`, `e`, hex-ish characters, `NUL` and `0xff`, plus a structured sweep of
  values around the `int`, `long` and small-integer ranges with assorted
  whitespace prefixes and suffixes. Zero differences.

## Confirmed unchanged

`c_src/` was read only. `git status` reports no modifications there.
