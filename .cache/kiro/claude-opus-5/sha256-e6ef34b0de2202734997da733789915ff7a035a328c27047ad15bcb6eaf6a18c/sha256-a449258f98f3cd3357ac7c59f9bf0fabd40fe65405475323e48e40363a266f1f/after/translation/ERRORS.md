# ERRORS.md — mismatches found while differentially testing the translation

Reference: `c_src/src/main.c`, built with CMake as `c_src/build/driver`
(gcc + glibc, x86-64 Linux).
Translation: `translation/src/main.rs`, built as `translation/target/release/driver`.

Method: both programs are run as subprocesses on identical stdin; stdout,
stderr and exit status are compared byte for byte
(`translation/tests/differential.rs`).

## What the C program actually branches on

The program has a single input-dependent decision:

```c
int x = 0;
scanf("%d", &x);
run(x);
run(x);
```

`run` is pure output plus mutation of the file-scope `the_house`, so every
input class is a class of the one `%d` conversion:

1. EOF before any non-whitespace byte → no assignment, `x` stays `0`.
2. Matching failure (optional sign not followed by a digit) → no assignment,
   `x` stays `0`.
3. Successful conversion in `int` range → `x` is that value.
4. Successful conversion out of `int` range but in `long` range → truncated to
   `int` by the store through `int *`.
5. Conversion out of `long` range → glibc saturates (ERANGE) before truncating.

There are no other `if`s, no early `return`s, no length or null checks, and no
path that writes to stderr or returns non-zero: `main` always `return 0`.

## Mismatch 1 — `scanf("%d")` wrapped instead of saturating on `long` overflow

Status: FIXED (`translation/src/main.rs`, `scanf_int`).

The original Rust `scanf_int` accumulated the digit run into an `i64` using
`wrapping_mul(10)` / `wrapping_add`, then cast to `i32`. glibc does not wrap.
Its `%d` directive converts through the same path as `strtol`: on overflow the
result saturates to `LONG_MAX` (or `LONG_MIN` for a negative sign) and `errno`
is set to `ERANGE`; that saturated `long` is then narrowed to `int` by plain
truncation when stored. On this platform `long` is 64-bit, so:

- overflowing positive input → `(int)LONG_MAX` → `-1`
- overflowing negative input → `(int)LONG_MIN` → `0`

Observed divergence (value of `x` inferred from the `bedrooms` field, which
starts at `5` and has `x` added to it once per `run` call):

| stdin | C `x` | Rust `x` (before fix) |
|---|---|---|
| `99999999999999999999` | `-1` | `1661992959` |
| `-99999999999999999999` | `0` | `-1661992959` |
| `18446744073709551616` (2^64) | `-1` | `0` |
| `1` repeated 40 times | `-1` | `-1908874354` |

Rendered difference for stdin `99999999999999999999` (line 4 of stdout):

```
C:    The house has 3 floors, 4 bedrooms, and 3.5 bathrooms
Rust: The house has 3 floors, 1661992964 bedrooms, and 3.5 bathrooms
```

Fix: accumulate the magnitude in a `u64` with `checked_mul` / `checked_add`,
latch an `overflow` flag when it no longer fits, compare against the
sign-dependent limit (`LONG_MAX` for `+`, `-LONG_MIN` for `-`), yield
`i64::MAX` / `i64::MIN` when out of range, and only then cast to `i32`.

Note that wrapping and saturating agree for inputs that overflow `int` but
still fit in `long` (`2147483648` → `-2147483648`, `4294967296` → `0`), so
those cases did not expose the bug; only inputs beyond `long` range did. Tests
`values_beyond_int_are_truncated_from_long` and
`values_beyond_long_saturate_then_truncate` now cover both sides of that edge.

## Mismatch 2 — test harness deadlock/EPIPE on large stdin (not a translation bug)

Status: FIXED in the test harness only; no change to `src/main.rs`.

The first version of the harness wrote stdin from the test thread and
`expect()`ed the write to succeed. Neither program drains stdin before exiting,
so a payload larger than the pipe buffer (~64 KiB) makes the write fail with
`BrokenPipe` once the child has exited. This is identical behavior for the C and
the Rust binary and is not observable in stdout/stderr/status, so the harness now
writes stdin on a helper thread and treats `BrokenPipe` as expected. The helper
thread also removes the possibility of deadlocking against the child's captured
stdout/stderr pipes.

The harness additionally runs every case twice — once with stdin as a pipe and
once with stdin as a regular file redirect — because a seekable stdin is a
different stdio configuration than a pipe.

## Behaviors verified as already matching (no fix needed)

- `%.1f` formatting of `bathrooms`. It only ever takes the values `2.5`, `3.5`
  and `4.5`, all exactly representable, so no rounding-mode difference between
  glibc and Rust's `{:.1}` can surface. The NaN/infinity handling in
  `format_f1` is unreachable from any input but is harmless.
- `%d` leading-whitespace skip, including across newlines (`scanf` semantics,
  unlike `fgets`): `"\n\n\n42"` and `"   42"` both yield `42`.
- Whitespace-only and empty input both leave `x == 0`.
- Matching failure cases: `"abc"`, `"-"`, `"+"`, `"--5"`, `"+ 5"`, `".5"`,
  `"e5"` — all leave `x == 0` and still exit `0`.
- Conversion stops at the first non-digit: `"12abc"` → `12`, `"0x10"` → `0`,
  `"1e5"` → `1`, `"5.5"` → `5`.
- Leading zeros are decimal, not octal, and a long run of them does not
  trigger the overflow path: `"007"` → `7`, 5000 zeros then `7` → `7`.
- Signed wraparound of `bedrooms` (`int`) across the two `run` calls, e.g.
  `2147483647` → `-2147483644` then `3`.
- `-0` → `0`.
- Binary stdin: leading NUL, bytes `0x80`–`0xff`, all 256 byte values,
  invalid UTF-8. Non-digit, non-whitespace bytes are a matching failure in both.
- Digit runs and whitespace runs straddling the Rust reader's 4096-byte
  buffer boundary (4094/4095/4096/4097/8192), including a sign landing exactly
  on the boundary.
- Only the first token is consumed; trailing input is never read.
- Exit status is `0` and stderr is empty for every input tried, including a
  200 000-byte payload.
- 400 randomized inputs over the alphabet `0-9 + - \n \t \r a b c x . NUL VT FF / :`
  and 150 randomized signed digit strings of length 1–25.

## Current state

- `c_src/` unmodified (only the ignored `c_src/build/` output directory is
  created).
- Both programs build with no errors and no warnings.
- `cargo test` passes in both debug and release profiles: 20 tests, 0 failed,
  0 ignored. No test is disabled, skipped or `#[ignore]`d.
