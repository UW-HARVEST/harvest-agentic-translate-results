# Differential verification log — C (`c_src/src/main.c`) vs Rust (`translation/src/main.rs`)

## What the C program does

```c
void driver(int x) { auto int y = 2*x; y += 300; printf("%d\n", y); }
int  main(void)    { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

There is exactly one branch point in the source: `scanf("%d", &x)` either
performs a conversion or fails. On failure (input failure at EOF / read error,
or a matching failure on a non-numeric byte) `x` keeps its initializer `0`, so
the program prints `300`. `main` never inspects `scanf`'s return value, always
returns 0, and never writes to stderr.

## Build and run commands

| | build | run |
|---|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` | `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` | `translation/target/release/driver` |

Both build with no errors and no warnings.

## Mismatches found and fixed

### 1. A failing `stdout` write aborted the Rust program

* **Symptom** — with stdout pointed at a device that fails every write
  (`/dev/full`, i.e. `ENOSPC`):
  * C: stdout empty, stderr empty, exit status **0**
  * Rust: stderr `thread 'main' panicked ... failed printing to stdout: No space
    left on device (os error 28)`, exit status **134** (`SIGABRT`, because the
    release profile sets `panic = "abort"`)
* **Cause** — `driver` used `println!`, which panics when the underlying write
  fails. C's `printf` merely returns a negative value, and `main` ignores it, so
  the failure is invisible and the exit status is unaffected.
* **Fix** — `driver` now formats the line and writes it with
  `io::stdout().write_all(...)`, discarding the `Result`. The exit-time
  `flush()` in `main` already discarded its error.
* **Regression test** — `failing_stdout_is_silent`.

### 2. A broken `stdout` pipe produced the wrong termination cause

* **Symptom** — when the reader of stdout closes the pipe before the program
  writes:
  * C: killed by **signal 13** (`SIGPIPE`), no output on either stream
  * Rust (after fix 1): exited **0**; Rust before fix 1: `SIGABRT` from the
    `println!` panic
* **Cause** — the Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs,
  so the write returns `EPIPE` instead of killing the process. A C program
  inherits the default disposition and is terminated by the signal.
* **Fix** — `main` calls `restore_default_sigpipe()` first, which resets
  `SIGPIPE` to `SIG_DFL` via `signal(2)`.
* **Regression test** — `broken_stdout_pipe_matches` (creates a pipe, hands the
  write end to the child as stdout, closes the read end, then unblocks the child
  by writing stdin).

Note that this mismatch was only observable *after* fixing #1; both defects lived
on the same `printf` call, so recording only the first would have hidden the
second.

## Behaviours that already matched, and were pinned down with tests

These were verified, not changed. They are listed because each is a place where
a "reasonable" Rust translation would diverge.

* **`scanf` skips whitespace across newlines.** `"\n\n\n\n42"` prints `384`;
  a `fgets`/`read_line`-based translation would have stopped at the first
  newline and printed `300`. Covered by
  `leading_whitespace_is_skipped_across_newlines` and
  `whitespace_only_input`. The whitespace set is C's `isspace` in the "C"
  locale: space, `\t`, `\n`, `\v`, `\f`, `\r`.
* **Matching failure leaves `x` untouched.** `"abc"`, `".5"`, `"-"`, `"+"`,
  `"- 5"`, `"--5"`, a leading NUL byte and a leading `0xff` byte all print
  `300`, because `scanf` returns without storing. Covered by
  `matching_failure_non_numeric` and `matching_failure_sign_without_digits`.
* **The conversion stops at the first non-digit and the remainder is never
  read.** `"0x10"` reads just `0` → `300`; `"5 6"` reads only `5` → `310`;
  `"1e5"` reads `1` → `302`. Covered by
  `conversion_stops_at_first_non_digit`.
* **`2*x` wraps as a 32-bit signed multiply.** `2147483647` → `2*x` wraps to
  `-2`, so the program prints `298`; `-2147483648` → `2*x` wraps to `0`, so it
  prints `300`. Rust uses `wrapping_mul`/`wrapping_add`; plain `*` would panic
  in a debug build and `saturating_mul` would print `2147483647`-derived
  garbage. Covered by `int_overflow_in_doubling`.
* **Out-of-`int`-range input is *truncated*, not clamped.** glibc converts `%d`
  in `long` range and the store truncates to `int`: `4294967296` (2^32) → `0` →
  `300`, `4294967295` → `-1` → `298`, `2147483648` → `INT_MIN` → `300`. Covered
  by `truncation_of_out_of_int_range_values`.
* **Input beyond `long` range saturates at `LONG_MAX`/`LONG_MIN` first, then
  truncates.** `9223372036854775808` and a run of 100 000 nines both saturate to
  `LONG_MAX`, whose low 32 bits are `-1`, so both print `298`; the negative
  forms saturate to `LONG_MIN`, whose low 32 bits are `0`, so both print `300`.
  Covered by `saturation_beyond_long_range` and `very_long_digit_runs`.
* **Unreadable stdin is an input failure.** With stdin on a directory (`read`
  fails with `EISDIR`) both print `300` and exit 0 — a read error is not
  distinguished from EOF. Covered by `unreadable_stdin_is_input_failure`.
* **`argv` is ignored.** Covered by `command_line_arguments_are_ignored`.
* **Output format.** `"%d\n"` — a bare decimal integer and exactly one trailing
  newline, no padding. Every test compares stdout byte for byte.

## Test suite

`translation/tests/differential.rs` — 16 tests, none `#[ignore]`d, skipped or
otherwise disabled. Every test spawns *both* binaries as subprocesses (the Rust
code is never loaded as a library) and asserts stdout, stderr and the exit
status all match, comparing the signal number as well as the exit code so a
signal death is never mistaken for a clean exit. The C binary is built by
`cmake` on first use if `c_src/build/driver` is absent.

`deterministic_random_sweep` adds 1 500 generated inputs from a fixed-seed LCG
over the alphabet `0-9 + - space \t \n \r \v \f a b c X Y Z . NUL 0xff / e`, so
any failure reproduces exactly.

## Harness sensitivity check

To confirm the suite measures something, three deliberate defects were injected
into `translation/src/main.rs` and the suite re-run; each was caught, and the
file was restored afterwards:

| injected defect | result |
|---|---|
| `+300` → `+301` | 14 of 16 tests failed |
| `wrapping_mul` → `saturating_mul` | 4 tests failed (`int_overflow_in_doubling`, `truncation_of_out_of_int_range_values`, `very_long_digit_runs`, sweep) |
| truncate-to-`int` → clamp-to-`int` | 2 tests failed (`truncation_of_out_of_int_range_values`, sweep) |

## Status

* Both programs build with no errors.
* `cargo test` passes in `translation/` (16 passed, 0 failed, 0 ignored), in
  both the debug and release profiles.
* An additional out-of-band sweep of 4 668 inputs (numeric boundaries plus
  random byte strings) found 0 differences.
* Nothing in `c_src/` was modified; only `c_src/build/` was created, by the
  documented build command.
