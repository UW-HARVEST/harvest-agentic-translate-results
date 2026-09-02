# Differential verification of the Rust translation

Reference: `c_src/src/main.c`, built with CMake.
Subject: `translation/src/main.rs`, built with Cargo.

Run commands recorded in Phase A:

```
# C reference
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
./c_src/build/driver              # reads stdin

# Rust subject
cd translation && cargo build --release
./translation/target/release/driver   # reads stdin
```

The comparison is done by running both executables as subprocesses with the same
bytes on stdin and comparing stdout, stderr, exit code and terminating signal.
The suite lives in `translation/tests/differential.rs` (24 tests); the Rust code
is never loaded as a library.

## What the C program branches on

`main()` does four unchecked `scanf` calls, then `driver(x, y, !!b, z)`. Every
input class therefore comes from `scanf` behaviour or from the bit-field widths
in `foo_t`:

| Class | Example input | Observable effect |
| --- | --- | --- |
| input failure (EOF) on conversion *n* | `""`, `"1"`, `"1 2 3"` | remaining variables keep their `= 0` initialisers |
| matching failure (non-numeric) | `"x 1 2 3"`, `"1,2,3,4"` | that variable and every later one stay 0, because the offending byte is left in the stream |
| sign with no digits | `"-"`, `"- 1 2 3"` | matching failure; the sign is consumed |
| `unsigned int x : 2` | `"4 0 0 0"` | stores value & 3 |
| `unsigned int y : 3` | `"0 8 0 0"` | stores value & 7 |
| `bool b : 1` via `!!b` | `"0 0 256 0"` | any non-zero prints as 1 |
| `int z` | `"0 0 0 -2147483648"` | printed with `%d`, no truncation |
| `%u` with a minus sign | `"-1 -1 -1 -1"` | `strtoul` negates mod 2^64, then truncated to `unsigned int` |
| range overflow | `"18446744073709551616 …"` | `strtoul`/`strtol` saturate at `ULONG_MAX`/`LONG_MAX`/`LONG_MIN` (64-bit), then truncate to 32 bits |
| whitespace spanning lines | `"1\n2\n3\n4\n"` | `%u`/`%d` skip newlines, so one conversion crosses lines |
| non-decimal prefixes | `"0x10 1 2 3"` | base 10 only: reads `0`, then fails on `x` |
| arbitrary bytes | `"\x00"`, `"\xff\xfe 1 2 3"` | ordinary non-numeric bytes |
| streaming stdin | `"1 2 3 4\n"` with stdin left open | prints and exits without waiting for EOF |
| stdout is a closed pipe | — | dies from `SIGPIPE` (wait status 141) |

## Mismatches found and fixed

### 1. Broken pipe produced the wrong exit status

* Symptom: with stdout connected to a pipe whose read end is closed, the C
  program was killed by `SIGPIPE` (no exit code, signal 13, shell reports 141)
  while the Rust program printed nothing and exited 0.
* Cause: the Rust standard library installs `SIG_IGN` for `SIGPIPE` before
  `main` runs. The failing `write` therefore surfaced as an `EPIPE` `io::Error`,
  which the translation discards (matching C's ignored `printf` return value),
  so the process ran to a normal `return 0`. A C program started from a shell
  inherits the *default* `SIGPIPE` disposition instead.
* Fix: `restore_default_sigpipe()` in `src/main.rs` calls `signal(SIGPIPE,
  SIG_DFL)` as the first statement of `main`, before any output is produced.
* Covered by: `broken_pipe_exit_status_matches`. Reverting the fix makes that
  test fail with `C (None, Some(13)) vs Rust (Some(0), None)`.

### 2. stdin was slurped to EOF instead of being read lazily

* Symptom: given `"1 2 3 4\n"` on a pipe that is *not* then closed, the C
  program printed `1 2 1 4` and exited 0 immediately; the Rust program hung
  until stdin was closed (observed as `timeout` status 124).
* Cause: the scanner was built with `std::io::stdin().read_to_end(&mut buf)`,
  so no conversion could start before end-of-input. `scanf` instead pulls bytes
  on demand and stops one byte past the digits it accepted, so the trailing
  newline is enough to finish the fourth conversion.
* Fix: `Scanner` in `src/main.rs` is now generic over `Read`, holds a single
  byte of lookahead (the byte `scanf` would have `ungetc`'d) and reads through a
  `BufReader` only when `peek()` has nothing buffered. `EINTR` is retried; any
  other read error becomes a sticky EOF, which is what a `FILE` error indicator
  does to the following conversions.
* Covered by: `stdin_held_open_does_not_block`, which feeds input in chunks and
  keeps the write end alive. Restoring the eager read makes it fail with
  `driver did not finish within 5s while stdin stayed open`.

### 3. Dead duplicate scanner removed (not a behavioural mismatch)

`src/scanf.rs` contained a second, slightly different `scanf` emulation that was
never declared as a module and therefore never compiled. It has been deleted so
that the only conversion logic in the crate is the one actually exercised.

## Behaviour deliberately reproduced, not "fixed"

* Return values of all four `scanf` calls are ignored, so a failed conversion is
  silent and the program still exits 0 and still prints four fields.
* A matching failure poisons every later conversion, because the offending byte
  is never consumed. `"1 x 3 4"` yields `1 0 0 0`, not `1 0 3 4`.
* `%u` accepts `-`. `"-1 -1 -1 -1"` prints `3 7 1 -1`: `x` and `y` receive
  `0xFFFFFFFF` truncated to 2 and 3 bits, `!!b` maps `-1` to 1, and `z` stays
  `-1`.
* Out-of-range magnitudes saturate at the *64-bit* limit before the 32-bit store,
  so `"1 2 9223372036854775808 9223372036854775808"` prints `1 2 1 -1`
  (`LONG_MAX` truncated to `int` is `-1`), and a negative overflow truncates
  `LONG_MIN` to `0`.
* `%d` values that merely exceed `int` are truncated, not clamped:
  `"1 2 4294967296 4294967296"` prints `1 2 0 0`.
* Output is exactly `"%u %u %d %d\n"`: single spaces, one trailing newline, no
  leading or trailing padding.
* Command-line arguments are ignored; nothing is ever written to stderr.

## Verification performed

* `cargo build --release` — clean, no warnings.
* `cargo test` and `cargo test --release` — 24 passed, 0 failed, 0 ignored. No
  test is `#[ignore]`d, skipped or disabled.
* An external differential fuzz of over 90,000 inputs (random byte soup over
  `0-9`, signs, all six C whitespace characters, `.`, `,`, `e`, `x`, NUL,
  `0xFF`, punctuation; plus token grids around 0, 3, 7, 8, 15, `INT_MAX`,
  `INT_MIN`, `UINT_MAX`, `LONG_MAX`, `ULONG_MAX` and 400-digit runs) found no
  remaining difference in stdout, stderr or exit status.
* Mutation testing confirmed the suite is not vacuous. Each of these injected
  defects was caught: widening the `x` mask to 3 bits, widening the `y` mask to
  4 bits, replacing `!!b` with `b & 1`, adding a trailing space to the format,
  dropping the trailing newline, printing the `bool` with Rust's `Display`,
  removing `%u` overflow saturation, ignoring the minus sign for `%u`, treating
  only `' '` as whitespace, dropping `LONG_MIN` saturation for `%d`, not
  consuming a leading `+`, clamping instead of truncating to `int`, reverting the
  `SIGPIPE` fix, and reverting the lazy-read fix.
* `c_src/src/main.c` and `c_src/CMakeLists.txt` are unmodified; only the
  generated `c_src/build/` directory was added.
