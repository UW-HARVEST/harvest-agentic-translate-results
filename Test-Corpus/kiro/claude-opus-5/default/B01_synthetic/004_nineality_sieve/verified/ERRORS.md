# Differential verification: `c_src` (C, ground truth) vs `translation` (Rust)

The C program (`c_src/src/main.c`) takes one integer argument and counts upward,
printing each value, until the value ends in 9 in base 10.

## How the two programs are run

| | command |
|---|---|
| C | `c_src/build/driver <arg>` (built with `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`) |
| Rust | `translation/target/release/driver <arg>` (`cd translation && cargo build --release`) |

`translation/tests/differential.rs` spawns both binaries as subprocesses and
compares stdout bytes, stderr bytes, and the exit status (both the exit code and
the terminating signal). The Rust code is never loaded as a library. The test
harness builds the C binary with CMake on first use if it is missing, so
`cargo test` is self-contained.

## Mismatches found and fixed

### 1. `SIGPIPE` was ignored, so the Rust program survived a closed stdout

*Symptom.* With stdout connected to a reader that goes away early, the two
programs terminated differently:

```
$ set -o pipefail; c_src/build/driver -2000000            | head -n 1  ; echo $?
-2000000
141          # killed by SIGPIPE (128 + 13)
$ set -o pipefail; translation/target/release/driver -2000000 | head -n 1 ; echo $?
-2000000
0            # exited normally
```

*Cause.* The Rust runtime sets the `SIGPIPE` disposition to `SIG_IGN` before
`main` runs. Writes to the broken pipe therefore returned `EPIPE` instead of
killing the process; because the C code never checks the return value of
`printf`, the translation deliberately discards write errors too, so the loop
simply ran to completion and exited 0. The C program keeps the default
disposition and is killed by the signal.

*Fix.* `restore_default_sigpipe()` in `translation/src/main.rs` resets `SIGPIPE`
to `SIG_DFL` as the first statement of `main`, declaring libc's `signal()`
directly so no new crate dependency is needed. Regression test:
`sigpipe_terminates_both_programs_the_same_way`, which asserts the C program is
killed by signal 13 and that the Rust program matches.

This is the only behavioural mismatch found between the two programs.

## Behaviours that had to be preserved and were confirmed correct

These are the C quirks the translation already reproduced. Each has a test; they
are listed because they are the places a translation is most likely to drift.

- **Errors go to stdout, not stderr.** The C code uses `printf` for both error
  messages, so stderr is empty on every path. Asserting only on stdout would
  have hidden the exit-code differences, so all three streams are compared.
- **`long` is narrowed to `int`.** `int val = strtol(...)` truncates. `4294967305`
  (2^32 + 9) prints just `9`; `4294967296` prints `0`..`9`.
- **`strtol` saturates rather than failing.** `99999999999999999999` saturates to
  `LONG_MAX` (`0x7fff_ffff_ffff_ffff`), whose low 32 bits are `-1`, so the output
  is `-1`..`9` and the exit code is 0. The negative side saturates to `LONG_MIN`,
  whose low 32 bits are `0`, giving `0`..`9`. The C code ignores `errno`, so the
  translation must not treat overflow as an error.
- **Only `end == argv[1]` is checked, not `*end == '\0'`.** Trailing garbage is
  accepted and discarded: `5abc`, `1e5`, `3.9`, `0x10` (which parses `0` in base
  10 and stops at `x`), `8,000` all succeed.
- **`strtol` skips leading C-locale whitespace** (space, `\t`, `\n`, `\v`, `\f`,
  `\r`) and an optional sign, but a string with no digits after that reports no
  conversion, so ` `, `\t`, `+`, `-`, `--5` and `  -  5` are all errors.
- **`%` truncates toward zero in C.** `val % 10 == 9` is therefore never true for
  a negative `val`: `-9` does not stop immediately, it counts all the way up to
  positive `9`. Rust's `%` has the same truncating semantics, so this maps
  directly, but a translation that reached for `rem_euclid` would break it.
- **Signed overflow wraps as the C build performs it.** `2147483647` (`INT_MAX`)
  ends in 7, so the loop increments past it; the C binary, built without
  optimisation, wraps to `-2147483648`. The translation uses `wrapping_add`.
  These inputs emit roughly 4.3 billion lines, so they are verified by comparing
  a bounded stdout prefix (where the wrap is directly observable, in the first
  two lines) plus the termination status, rather than the whole stream.
- **Non-UTF-8 arguments.** `std::env::args_os()` is used rather than `args()`,
  which would panic; a `\xff\xfe` argument produces the same parse error as in C.
- **`argc == 0`.** Verified manually with `perl -e 'exec {$ARGV[0]} ()'`: both
  programs take the `argc != 2` branch and exit 1. This is not in the automated
  suite because `std::process::Command` cannot spawn a process with an empty
  `argv`; `argc == 1`, `2`, `3` and `5` are covered there instead.

## Traps hit while writing the tests (harness errors, not translation defects)

Recorded because both produced misleading "everything matches" results:

1. A shell sweep read `${PIPESTATUS[0]}` after a command substitution, so it was
   reporting the status of the assignment rather than of the program under test.
   Every case reported status 0, making the exit-status comparison vacuous. The
   Rust suite reads the status from `Child`/`Output` directly.
2. A Python fuzz driver used Python's floored `%` to decide whether an input
   terminates quickly. Python gives `-2147483641 % 10 == 9` while C gives `-1`,
   so ~1700 multi-billion-iteration inputs were mistakenly run with full output
   capture and aborted. Rerun with truncating modulo: 5612 cases, 0 mismatches,
   plus 130 wrap-region cases compared by prefix, 0 mismatches.

## Result

- Both programs build with no errors and no warnings.
- `cargo test` in `translation/`: 23 tests, all passing, none `#[ignore]`d,
  skipped or disabled.
- Randomised differential fuzzing: 5612 full-output cases and 130 wrap-region
  prefix cases, 0 mismatches.
- Nothing in `c_src/` was modified; the only addition is the `c_src/build/`
  directory produced by the prescribed CMake command.
