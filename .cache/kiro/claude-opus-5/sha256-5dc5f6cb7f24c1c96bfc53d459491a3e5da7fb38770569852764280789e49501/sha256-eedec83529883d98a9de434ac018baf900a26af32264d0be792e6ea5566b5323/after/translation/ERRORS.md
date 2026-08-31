# ERRORS.md — differential verification of `c_src` vs. `translation`

Ground truth: `c_src/src/main.c`, built with CMake (`c_src/build/driver`).
Candidate: this crate's `driver` binary.
Comparison: both run as subprocesses on identical stdin; stdout, stderr and
exit status (including termination by signal) compared byte for byte.
See `tests/differential.rs`.

The whole program is four statements, so every branch lives inside libc:

```c
int x = 1, y = 1;
scanf("%d %d", &x, &y);
div_t result = div(x, y);
printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
```

The input classes therefore are: how many `%d` conversions succeed, what
`strtol` does at its range limits, and whether `idiv` traps.

## Mismatches found

### 1. Interrupted reads were treated as end of input — FIXED

`Scanner::next_byte` mapped every `io::Error` from `Read::read`, including
`ErrorKind::Interrupted` (`EINTR`), to end of input. Once that happened the
scanner latched `eof = true` and refused to read again, so a signal arriving
mid-read would abandon a conversion that glibc would have completed. Concretely,
input `12 3` delivered while a signal handler fires could yield
`quotient: 1, remainder: 0` (both operands left at their initialisers) where the
C program yields `quotient: 4, remainder: 0`.

Cause: glibc's stdio restarts a `read(2)` that returns `EINTR`; a bare
`Read::read` in Rust surfaces it to the caller instead. Only `read_to_end` and
friends retry.

Fix: `next_byte` now loops on `ErrorKind::Interrupted` and keeps reading.
`Ok(0)` and genuine errors still mean end of input.

This is a latent divergence rather than one a test reproduced — the tests here
cannot deliver a signal to the child at the right instant. It was found by
reading the reader against glibc's behaviour, and `stdin_delivered_in_slow_chunks`
covers the neighbouring case (a short read must not look like EOF).

## Behaviours checked and confirmed already correct

No stdout, stderr or exit-status difference was observed on any other input.
7500 randomised inputs (random bytes, scanf-alphabet soup, operands jittered
around every 32- and 64-bit boundary, long digit runs, whitespace soup)
produced zero mismatches, as did the enumerated suite. Specifically confirmed:

- **`scanf` return value is discarded.** A conversion that never happens leaves
  its variable at `1`. Empty input, whitespace-only input and `xyz` all print
  `quotient: 1, remainder: 0`; `42` alone divides by the default `y == 1`.
- **`%d` reads across newlines.** `\n\n\n5\n\n\n2` is `5 / 2`, not a failed
  read. Whitespace skipping covers exactly the C-locale `isspace` set
  (`' '`, `\t`, `\n`, `\v`, `\f`, `\r`); `0x85` and `0xa0` are not whitespace,
  and `LC_ALL` cannot change that because the program never calls `setlocale`.
- **Ordering of the two conversions.** `scanf` stops at the first failure, so a
  bad `x` means `y` is never even attempted. `abc 10 3` prints
  `quotient: 1, remainder: 0` — the `10` is not consumed into `y`.
- **One byte of lookahead is pushed back.** `10 3abc`, `5,2`, `0x10 2`, `3.9 2`
  and `5e2 3` all stop the conversion at the offending byte.
- **strtol saturation, then truncation into `int`.** Out-of-range literals clamp
  to `LONG_MAX`/`LONG_MIN` and are then truncated: `4294967297` → `1`,
  `4294967295` → `-1`, `9223372036854775807` → `-1`,
  `-9223372036854775808` → `0`, and a 1000-digit run of `9`s → `-1`
  (`0` when negated). Leading zeros and a `+` sign do not change any of this.
- **`div` truncates toward zero,** so the remainder takes the sign of the
  dividend: `-7 / 2` is `-3` remainder `-1`, and `7 / -2` is `-3` remainder `1`.
  Verified over the full grid of operands in `-6..=6`.
- **Undefined division traps rather than returning.** `div(x, 0)` and
  `div(INT_MIN, -1)` reach `idiv` with no representable result, so the process
  dies of `SIGFPE` (no exit code, empty stdout, empty stderr). This includes
  divisors that only become zero after truncation, e.g. `4294967296` and
  `18446744073709551616`. The Rust side reproduces it by executing the same
  `idiv` instruction; a checked or wrapping division would exit `0` instead and
  is caught by `division_by_zero_traps` and `int_min_over_minus_one_traps`.
- **`printf` formatting**, including the `", "` separator and exactly one
  trailing newline.
- **`argv` is ignored** (`main` takes no parameters), and stdin from
  `/dev/null`, from a closed descriptor, and delivered in 20 ms chunks all
  behave identically.

## Test-suite validation

To confirm the suite would actually catch a regression rather than passing
vacuously, fourteen deliberate defects were injected into `src/main.rs` and the
suite re-run. Thirteen were caught:

| Injected defect | Caught |
| --- | --- |
| `y` initialised to `0` instead of `1` | yes |
| failed conversion stores `0` instead of leaving the initialiser | yes |
| second `%d` attempted after the first fails | yes |
| `\n`, `\v`, `\f`, `\r` dropped from `isspace` (`fgets`-like reading) | yes |
| lookahead byte discarded instead of pushed back | yes |
| no saturation; accumulator allowed to run past `LONG_MAX` | yes |
| truncation taken from the wide accumulator, skipping the `long` clamp | yes |
| `div` instead of `idiv` (unsigned division) | yes |
| Euclidean instead of truncating division | yes |
| divide-by-zero returns `0 0` instead of trapping | yes |
| `INT_MIN / -1` wraps instead of trapping | yes |
| trailing newline removed from the format string | yes |
| `exit(1)` after printing | yes |
| saturation threshold `LONG_MAX` instead of `LONG_MAX + 1` | no — equivalent |

The surviving mutant is not a coverage gap. The two thresholds differ only when
the accumulator lands on exactly `2^63`, and in that case the saturating branch
and the post-loop clamp both produce `LONG_MAX` (or `LONG_MIN` when negated),
so no input can distinguish them.

## Not covered

- Non-x86-64 targets. `c_div` falls back to Rust's `/` and `%` there, which
  panic instead of raising `SIGFPE`, so the divide-by-zero and `INT_MIN / -1`
  cases would not match. Verification was done on x86-64, where the `idiv` path
  is used.
- The exact instant of a signal arriving mid-`read`, which is what defect 1
  above concerns.
- Behaviour when stdin is a terminal.
