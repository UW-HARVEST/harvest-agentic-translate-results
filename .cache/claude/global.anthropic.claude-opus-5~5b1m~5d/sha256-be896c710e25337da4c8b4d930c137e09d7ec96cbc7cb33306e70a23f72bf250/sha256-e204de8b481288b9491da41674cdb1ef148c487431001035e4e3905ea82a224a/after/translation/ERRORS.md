# Differential verification log — `c_src/src/main.c` vs `translation/`

Ground truth is the C program. Everything below records a behavioural
difference that was observed by *running* both binaries and diffing stdout,
stderr and exit status, plus what caused it and how the Rust side was changed.

## How the two programs are run

```
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
./c_src/build/driver            # reads stdin

# Rust
cd translation && cargo build --release
./translation/target/release/driver   # reads stdin
```

The test suite (`translation/tests/differential.rs`) spawns both as
subprocesses, feeds identical bytes on stdin, and asserts stdout, stderr,
exit code and terminating signal all match. It never links the Rust code as a
library.

## What the C program branches on

`main()` is `int x = 0; scanf("%d", &x); run(x); run(x);` — the return value of
`scanf` is discarded. So the reachable input classes are:

| Class | C behaviour |
|---|---|
| EOF before any input (empty / closed stdin) | `scanf` returns `EOF`, `x` stays `0` |
| whitespace only (`" "`, `"\n\n"`, `\t\r\v\f`) | `%d` skips it, then hits EOF, `x` stays `0` |
| leading whitespace then digits | `%d` skips **across newlines**, converts |
| sign then digits (`+7`, `-3`) | converted |
| sign with no digit (`-`, `+`, `-a`, `- 5`, `--5`) | matching failure, `x` stays `0` |
| non-numeric first char (`abc`, `.`, `!!!`) | matching failure, `x` stays `0` |
| digits then junk (`12abc`, `0x10`, `3.7`) | converts the digit prefix only |
| more than one token (`3 99`) | only the first is ever read |
| `INT_MAX` / `INT_MIN` | `bedrooms += x` twice ⇒ signed overflow (wraps in practice) |
| values beyond `int` / beyond `long` | glibc `%d` saturates at `long` range, then truncates to `int` |
| stdout write fails / reader closed | see mismatches #1 and #2 below |

`run()` itself is branch-free: 4 prints, `floors++`, `bathrooms += 1.0`,
`bedrooms += x` — called twice, so 8 lines of output always.

---

## Mismatch #1 — a failing `printf` aborted the Rust program

**Found with:** stdin `"3"`, stdout redirected to `/dev/full` (every write
fails with `ENOSPC`).

| | exit status | stderr |
|---|---|---|
| C | `0` | empty |
| Rust (before fix) | `134` (`SIGABRT`) | `thread 'main' panicked … failed printing to stdout: No space left on device (os error 28)` |

**Cause.** The translation used the `print!` macro. `print!`/`println!` **panic**
when the underlying write fails. C's `printf` merely returns a negative value,
and `print_the_house()` ignores it, so a failed write is completely silent and
the program still runs to `return 0`. With `panic = "abort"` in the release
profile the panic became a `SIGABRT`, so both the exit status *and* stderr
differed.

**Fix.** `src/main.rs`: `print_the_house()` now locks stdout and uses
`write!(...)` with the `Result` explicitly discarded (`let _ = write!(…)`),
matching C's ignore-the-return-value behaviour. The final flush in `main` was
already error-ignoring.

**Covered by:** `stdout_write_error_is_silent_like_printf`.

## Mismatch #2 — Rust survived a closed stdout pipe that killed the C

**Found with:** stdin `"3"`, stdout connected to a pipe whose read end is
closed before the child writes.

| | terminating signal | exit code | stderr |
|---|---|---|---|
| C | `SIGPIPE` (13) | — | empty |
| Rust (before fix) | none | `134` (abort, via the panic in #1) | panic message |

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main`
runs. A C program launched from a shell inherits the *default* disposition and
is therefore killed by `SIGPIPE` on the first write to a reader-less pipe. Even
after fixing #1 the Rust program would have exited `0` where the C is killed by
signal 13 — a difference a stdout-only test never sees.

**Fix.** `src/main.rs`: `restore_default_sigpipe()` calls `signal(SIGPIPE,
SIG_DFL)` (declared directly via `extern "C"`, no new dependency) as the first
statement of `main` on unix. Both programs now terminate with `SIGPIPE` and an
empty stderr.

**Covered by:** `closed_stdout_pipe_produces_the_same_termination`.

---

## Behaviours deliberately replicated rather than "fixed"

These are not mismatches — they are C quirks the Rust side reproduces on
purpose, and each has a test.

* **`scanf` reads across newlines.** `%d` skips *any* run of whitespace,
  newlines included, so `"\n\n\n42"` yields `42`. (`fgets` would not; the C
  does not use `fgets`.) — `leading_whitespace_is_skipped_across_newlines`
* **`scanf`'s return value is ignored.** On matching failure or EOF, `x` keeps
  its initialiser `0` and the program prints the same output as for input `0`.
  It never reports an error and never exits non-zero. There is *no* error path
  that changes the exit status. — `matching_failure_leaves_x_untouched`,
  `sign_with_no_digits_is_a_matching_failure`
* **One byte of pushback.** `%d` consumes the terminating non-digit and ungets
  it. Unobservable here (nothing else reads stdin) but modelled anyway so the
  digit prefix is cut in exactly the right place.
* **Signed `int` overflow.** `bedrooms += x` runs twice; with `x = INT_MAX`
  this overflows, which is UB in C but wraps on the target. The Rust uses
  `wrapping_add` so it wraps identically instead of panicking in debug builds.
  Same for `floors++`. — `int_boundaries_and_overflow_in_add_bedrooms`
* **Out-of-range integer literals.** glibc's `%d` accumulates with `strtol`
  semantics: it saturates at `LONG_MIN`/`LONG_MAX` and *then* truncates to
  `int`. So `"9223372036854775808"` ⇒ `LONG_MAX` ⇒ `-1`, and
  `"18446744073709551616"` also saturates rather than wrapping mod 2^64. The
  Rust `scanf_i32` accumulates in `i128`, clamps to `i64`, then casts to `i32`
  to reproduce this exactly. — `values_beyond_int_are_truncated_the_way_glibc_does`,
  `absurdly_long_digit_run` (5 000-digit inputs)
* **`%.1f` formatting.** `bathrooms` only ever takes values `2.5, 3.5, 4.5,
  5.5` — exactly representable, so C's round-half-to-even and Rust's `{:.1}`
  agree. The trailing `\n` on every line and the exact wording/spacing of
  `"The house has %d floors, %d bedrooms, and %.1f bathrooms\n"` are asserted
  byte for byte. — `output_shape_matches_printf_exactly`
* **Stdout buffering.** C's stdout is fully buffered to a pipe and flushed at
  exit; Rust's is line buffered. Since nothing is ever written to stderr, the
  byte streams are identical either way.
* **`int main()` takes no arguments**, so command-line arguments are ignored by
  both.

## Verification performed

* `cargo build --release` — clean, no warnings.
* `cargo test` and `cargo test --release` — 26 tests, all pass, none
  `#[ignore]`d, skipped or disabled.
* An additional out-of-band sweep of **2 195** inputs (boundary values around
  `2^31`, `2^63`, `2^64`, `10^20`, `10^30`; random `0-9 + - . , x a b Z` and
  whitespace soup; random raw byte strings including NULs and invalid UTF-8)
  produced **0** differences in stdout, stderr or exit status. The driver for
  this sweep is `fuzz_check.py` at the repository root.
* Nothing in `c_src/` was modified. The only thing created under it is the
  CMake `build/` directory from the documented build command; the test harness
  itself prefers an out-of-tree build at `translation/target/c_build` and never
  writes into `c_src/`.
