# Differential testing findings

The C program (`c_src/src/main.c`) reads one `float` with `scanf("%f", &x)`,
`memcpy`s it into a `char[4]`, and prints those four bytes as `%02x` each
followed by `'\n'`. `x` is initialised to `0.f`, so a `scanf` matching failure or
EOF leaves `00000000` as the output. `main` always `return 0`.

Both programs were run as subprocesses over every input class in
`tests/differential.rs` plus ~12,000 additional generated inputs, comparing
stdout, stderr and exit status (including death by signal) byte for byte.

## Mismatches found and fixed

### 1. SIGPIPE on a broken stdout pipe — Rust exited 0 where C died by signal

**Symptom.** With the read end of stdout's pipe already closed, the two programs
disagreed on how they terminated:

| program | result |
| --- | --- |
| C | killed by `SIGPIPE` (signal 13, shell reports 141) |
| Rust (before fix) | `exit(0)`, empty stdout, empty stderr |

**Cause.** The Rust standard library installs `SIG_IGN` for `SIGPIPE` before
`main` runs. With the signal ignored, the `write` in `print_hex` returns `EPIPE`
instead of raising the signal; the translation discarded that `io::Error` (as it
must, since C's `printf` return value is also discarded) and fell off the end of
`main` with status 0. The C program never touches the `SIGPIPE` disposition, so
it inherits `SIG_DFL` and is killed.

Note that a stdout-length check alone would not have caught this: both programs
produce no stdout in this scenario. Only the exit status differs, which is why
the test asserts all three streams.

**Fix.** `restore_default_sigpipe()` in `src/main.rs` calls `signal(SIGPIPE,
SIG_DFL)` as the first statement of `main`, via an `extern "C"` declaration of
`signal` (already linked into every Rust binary, so no new crate dependency).
Regression test: `broken_stdout_pipe_dies_the_same_way`.

## Behaviours confirmed identical (no fix needed)

These were all checked because they are the places a translation of
`scanf("%f")` usually drifts. Each one already matched.

- **`%f` skips leading whitespace across newlines.** `"\n\n\n\n4.25"` and
  5000-byte whitespace runs behave the same; a whitespace-only input is a
  matching failure and prints `00000000`.
- **Matching failure keeps the initialiser.** Empty input, `"abc"`, a lone `"-"`
  or `"+"`, `"."`, and every single byte 0x00–0xff all print `00000000` with
  status 0 in both.
- **Truncated `infinity` is a matching failure, not `inf`.** glibc accepts `inf`
  and `infinity` but rejects the intermediate prefixes: `"infi"`, `"infin"`,
  `"infini"`, `"infinit"` all yield `00000000`, while `"infinityx"` yields
  infinity. The translation reproduces this, including case-insensitivity.
- **Truncated `nan`.** `"n"`, `"na"` fail; `"nan"`, `"nan()"`, `"nan(abc_1)"`
  succeed; an unterminated `"nan("` or `"nan(abc"` still yields the default quiet
  NaN. The payload in the parentheses is consumed but does not reach the value —
  glibc produces `7fc00000` (`ffc00000` when signed) regardless.
- **`0x` with no hex digits is a matching failure.** `"0x"`, `"0X"`, `"0xg"`,
  `"0x."` all print `00000000` rather than the `0` that `strtof`'s
  longest-valid-prefix rule would give.
- **An incomplete exponent is dropped, not fatal.** `"1e"`, `"1e+"`, `"1e-x"`,
  `"0x1p"`, `"0x1p+"` keep the mantissa: `"1e"` is `1.0f`.
- **Rounding is round-half-to-even in both.** Verified on exact ties built as
  `(2m+1) * 2^(e-1)` for odd 25-bit significands across the whole exponent
  range, on `16777216`–`16777220`, and on `1.000000059604644775390625`.
- **Overflow, underflow and subnormals.** The largest finite float and the value
  just past it, the smallest normal, the largest and smallest subnormal, and
  exactly half the smallest subnormal (`7.00649232162408535e-46`, a tie to zero)
  all agree, for both signs.
- **Significands longer than any accumulator.** 200 000-digit decimal
  significands, 100 000-digit hex significands, and hex inputs exceeding the
  translation's 124-bit accumulator (where the discarded tail survives only as a
  sticky bit) all agree.
- **Absurd exponents.** `1e999999999999999999999`,
  `0x1p9223372036854775807` and their negatives saturate identically; the
  translation clamps the exponent at ±1 000 000, which is past the point where
  the result is already infinity or zero.
- **Signed zero and byte order.** `-0.0` prints `00000080`, confirming the
  little-endian `memcpy` of the sign bit is reproduced.
- **Output shape.** Always exactly 8 lowercase hex digits and one `'\n'`; stderr
  is always empty; status is always 0 on any normal run.
- **`argv` is ignored** — `main()` takes no parameters, so extra arguments change
  nothing in either program.
- **Unwritable and unreadable standard streams.** stdout redirected to
  `/dev/full` (write fails with `ENOSPC`, not `EPIPE`) still exits 0 in both, as
  does stdin from `/dev/null` or from a directory.

## Test suite integrity

The suite was mutation-checked to confirm it can actually fail. Each of these
deliberate regressions, applied one at a time and then reverted, was caught:

| mutation | tests that failed |
| --- | --- |
| drop the round-half-to-even condition | 5 |
| accept truncated `infinity` as `inf` | 1 (`infinity_forms_and_prefixes`) |
| remove the `SIGPIPE` fix | 1 (`broken_stdout_pipe_dies_the_same_way`) |
| print the bytes big-endian | 18 |

No test is `#[ignore]`d, skipped or disabled. Nothing in `c_src/` was modified.
