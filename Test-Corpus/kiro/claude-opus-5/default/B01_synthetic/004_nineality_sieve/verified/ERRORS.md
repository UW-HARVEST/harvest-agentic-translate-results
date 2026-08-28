# Differential verification log — `c_src/src/main.c` vs `translation/`

## What the C program does

```c
if (argc != 2)            -> printf("Error: should only be a single (integer) argument!\n"); return 1;
int val = strtol(argv[1], &end, 10);
if (end == argv[1])       -> printf("Error: first argument must be an integer!\n");        return 1;
while (1) { printf("%d\n", val); if (val % 10 == 9) break; val++; }
return 0;
```

Both error messages go to **stdout** (`printf`, not `fprintf(stderr, ...)`); the
program never writes to stderr on any path. Exit codes are only 0 and 1, plus
death by signal when stdout is closed.

## Build / run commands

| | command |
|---|---|
| C build | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` |
| C run | `c_src/build/driver <arg>` |
| Rust build | `cd translation && cargo build --release` |
| Rust run | `translation/target/release/driver <arg>` |
| Tests | `cd translation && cargo test` |

## Mismatches found in the Rust translation

**None.** Every input class enumerated below produced byte-identical stdout,
byte-identical stderr and an identical exit status (or identical terminating
signal). This was confirmed by the 14 tests in `tests/differential.rs` plus two
throwaway fuzz sweeps run outside the suite: 3754 random argument strings over
an alphabet of digits, signs, whitespace, letters, punctuation and invalid UTF-8
bytes, and 754 numeric-boundary values clustered around 2^31, 2^32, 2^63, 2^64
and 10^25. Zero mismatches in either sweep.

## Behaviors that would have been mismatches, and how the Rust matches them

These are the traps in this program. Each is exercised by a named test; each is
a place where a straightforward Rust translation diverges from the C.

1. **`long` -> `int` truncation.** `int val = strtol(...)` narrows a 64-bit
   `long`. `4294967296` (2^32) therefore starts the count at `0`, and
   `4294967291` starts it at `-5`. Rust reproduces this with `parsed as i32`
   (a wrapping cast). A translation that parsed straight into `i32` and errored
   on out-of-range input would print an error where C prints numbers.
   Test: `long_to_int_truncation`.

2. **`strtol` saturation, not failure.** On overflow `strtol` returns
   `LONG_MAX`/`LONG_MIN` and still consumes the whole digit run, so `end !=
   argv[1]` and the "must be an integer" branch is *not* taken.
   `9223372036854775808` saturates to `LONG_MAX`, whose low 32 bits are
   `0xFFFFFFFF`, so the program prints `-1, 0, ... 9`. `-99999999999999999999`
   saturates to `LONG_MIN`, whose low 32 bits are `0`, so it prints `0 ... 9`.
   `Rust`'s `strtol_base10` clamps to `i64::MIN`/`i64::MAX` while continuing to
   consume digits. Rust's own `str::parse` would have returned `Err` here.
   Tests: `strtol_saturation_then_truncation`, `very_long_digit_runs`.

3. **Truncating `%` on negative values.** In C, `-9 % 10 == -9`, never `9`, so a
   negative start never terminates early: it counts up through zero all the way
   to `+9`. Rust's `%` is also truncating, so `val % 10 == 9` transfers
   verbatim. A translation using a Euclidean remainder (`rem_euclid`) would stop
   at `-1` for input `-9...1` and produce far less output.
   Test: `negative_starts_count_through_zero`.

4. **Signed overflow on `val++`.** `2147483647` does not end in 9, so `val++`
   overflows. That is UB in C, but the compiled binary wraps to `INT_MIN` and
   the loop keeps counting to `+9` — roughly 2^31 lines of output. Rust uses
   `wrapping_add(1)`; plain `+ 1` would panic in a debug build and produce a
   different exit status and stderr. Because the full output is tens of
   gigabytes, this is verified by comparing a 64 KiB stdout prefix from both
   processes rather than the whole stream.
   Test: `signed_overflow_wraparound_prefix`.

5. **`strtol`'s prefix grammar.** Leading `isspace()` bytes are skipped, one
   optional sign is allowed, and conversion stops at the first non-digit without
   being an error. So `"  42"` -> starts at 42, `"12abc"` -> starts at 12,
   `"0x10"` -> starts at 0 (base 10 stops at `x`), `"1e3"` -> starts at 1, and
   `"1 2"` -> starts at 1. Only a completely absent digit run (`""`, `" "`,
   `"+"`, `"-"`, `"abc"`, `".5"`, `"--5"`, `"  -  9"`) leaves `end == argv[1]`
   and takes the error branch. `strtol_base10` returns an `end_index` of 0
   exactly in that case, mirroring "end is reset to the start of the string".
   Tests: `no_conversion_performed`, `counts_up_to_terminator`,
   `breaks_immediately`.

6. **Non-UTF-8 argv.** `argv` is bytes, not text. `"\xff"` takes the error
   branch and `"5\xff"` starts at 5. Rust reads the argument through
   `args_os()` + `OsStrExt::as_bytes()` rather than `args()`, which would panic
   on invalid UTF-8 and change both stderr and the exit status.
   Test: `non_utf8_arguments`.

7. **SIGPIPE disposition.** The C program leaves `SIGPIPE` at `SIG_DFL`, so a
   reader that closes the pipe early kills it (shell status 141). The Rust
   runtime sets `SIGPIPE` to `SIG_IGN`; combined with the ignored write results
   (`let _ = write!(...)`), an unpatched Rust build would silently churn through
   billions of failed writes and exit 0. `restore_default_sigpipe()` puts the
   disposition back to `SIG_DFL`. Verified: both die from signal 13.
   Test: `closed_stdout_kills_both_the_same_way`.

8. **Error text goes to stdout, and stderr stays empty.** Every test asserts
   stderr byte-for-byte, so a translation using `eprintln!` for the error
   messages would fail even though its exit code was right.
   Tests: `wrong_argument_count`, `no_conversion_performed`.

## Defects found and fixed — in the test harness, not the translation

Recorded because they were real failures during this run, and because both are
easy to reintroduce:

- `long_to_int_truncation` originally included `"2147483648"`. That truncates to
  `INT_MIN` and prints ~2^31 lines; capturing it with `Command::output()`
  buffered the whole stream in memory and the test binary was `SIGKILL`ed by the
  OOM killer. The case moved to the bounded-prefix test.
- `signed_overflow_wraparound_prefix` originally included `"2147483000"` and
  `"9223372036854775806"`. Those terminate after 10 and 12 lines respectively,
  so the "read 64 KiB" helper hit EOF and the `c.len() == N` assertion failed.
  Replaced with arguments that genuinely produce unbounded output.

Neither was a translation bug; the Rust binary's behavior was correct in both
cases.

## Inputs covered

- `argc`: 1, 3, 4, 11 arguments (all -> argument-count error, exit 1)
- no-conversion: empty string, each `isspace()` byte alone, mixed whitespace,
  bare `+`/`-`, doubled signs, sign detached from digits, leading `.`, letters,
  the bytes adjacent to `'0'`/`'9'` (`/` and `:`), a non-ASCII digit, and
  invalid UTF-8 byte sequences
- immediate break: `9`, `19`, `29`, `99`, `109`, `1000000009`, `2147483639`,
  plus forms with leading whitespace, `+`, leading zeros and trailing junk
- counting loop: `0`,`1`,`2`,`5`,`8`,`10`,`11`,`20`,`42`,`100`,`2147483630`
- negatives: `-1`,`-2`,`-5`,`-9`,`-10`,`-12`,`-19`,`-29`,`-100`,`-0009`
- truncation: 2^32, 2^32±k, 2^33, 3·2^32+9, 2^32−1, and negatives of those
- saturation: `LONG_MAX`−1/`LONG_MAX`/`LONG_MAX`+1, `LONG_MIN`/`LONG_MIN`−1,
  20- and 39-digit values, 5000-digit runs, 5000 leading zeros
- unbounded output: `2147483647`, `2147483648`, `-2147483648`, `-2000000000`,
  `-2147483639` (64 KiB prefix comparison)
- exhaustive sweep: every integer in `-60..=60`, and `1230..=1245`
- closed stdout mid-stream (SIGPIPE)

## Status

- Both programs build with no errors and no warnings.
- `cargo test` passes in both the dev and release profiles: 14 passed, 0 failed,
  0 ignored. No test is `#[ignore]`d, skipped or disabled.
- `c_src/` sources are unmodified. The only thing written under `c_src/` is the
  out-of-source `build/` directory produced by CMake, per the documented build
  command. `tests/differential.rs::c_sources_are_not_modified` asserts the C
  source still contains its original message strings and loop condition.
