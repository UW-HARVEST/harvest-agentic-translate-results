# Differential testing report

Verification of the Rust translation in `translation/` against the C ground
truth in `c_src/`.

## What the C program is

`c_src/src/main.c` is six lines of logic:

```c
int x = 1, y = 1;
scanf("%d %d", &x, &y);
div_t result = div(x, y);
printf("quotient: %d, remainder: %d\n", result.quot, result.rem);
return 0;
```

The `scanf` return value is discarded, so the branching is implicit rather than
written out. The input classes are:

| Class | Reached by | Result |
|---|---|---|
| both conversions succeed | `7 2` | normal output |
| first succeeds, second fails | `7`, `7 abc` | `y` keeps its initializer `1` |
| first fails | ``(empty)``, `abc`, `-` | both keep `1` and `1`, printing `quotient: 1, remainder: 0` |
| field out of `int` range | `2147483648 1` | glibc converts with `strtol` then narrows to `int` |
| field out of `long` range | `999999999999999999999999 2` | saturates at `LONG_MAX`/`LONG_MIN`, then narrows |
| divisor is `0` | `5 0` | `div()` traps: killed by `SIGFPE`, no stdout |
| `INT_MIN / -1` | `-2147483648 -1` | `div()` traps: killed by `SIGFPE`, no stdout |
| stdout is a closed pipe | `printf` fails | killed by `SIGPIPE` |

## How it is tested

`translation/tests/differential.rs` spawns **both compiled binaries** as
subprocesses on identical stdin and compares stdout bytes, stderr bytes, and the
exit status (exit code *and* terminating signal, kept distinct: `Ok(code)` vs
`Err(signum)`). The Rust code is never linked as a library.

- Rust binary: `env!("CARGO_BIN_EXE_driver")`.
- C binary: `c_src/build/driver`, built on demand with `cmake .. && cmake
  --build .` into the out-of-source `build/` directory. No file in `c_src/` is
  modified.

30 test functions, roughly 820 distinct inputs, including an exhaustive sweep of
every operand pair in `-12..=12` and a sweep over `int`/`long` boundary values.

## Mismatches found

### 1. `SIGPIPE` was ignored, so the Rust program survived a closed stdout

**Symptom.** With stdout wired to a pipe whose read end is already closed:

| | exit code | signal |
|---|---|---|
| C | — | 13 (`SIGPIPE`) |
| Rust (before fix) | 0 | — |

stdout and stderr were both empty in each case, so **a test that compared only
stdout would have passed.** Only the exit status exposed it.

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` in its runtime
setup before calling `main`. A C program inherits the default disposition. With
the signal ignored, the failing `write` returns `EPIPE`, the translation's
`let _ = write!(...)` discards that error, and `main` returns normally with
status 0. The C program is killed by the signal mid-`write`.

**Fix.** `restore_default_sigpipe()` in `src/main.rs` calls `signal(SIGPIPE,
SIG_DFL)` as the first statement of `main`, restoring the disposition the C
program runs with.

**Regression check.** Commenting out that call makes
`closed_stdout_pipe_raises_sigpipe` fail with
`left: ([], None, Some(13)), right: ([], Some(0), None)`; restoring it makes the
test pass. The test also pins the absolute expectation (signal 13, not merely
"both agree"), so it cannot pass by both programs exiting 0.

### 2. (test defect, not a translation defect) wrong hardcoded expectation

`stdin_from_a_regular_file` initially asserted that `9 4` yields
`remainder: 0`. The correct value is `remainder: 1`. The two programs agreed
with each other; only my literal was wrong. Corrected in the test.

## Behaviors deliberately preserved, not "fixed"

These all look like bugs and are reproduced exactly. Each was confirmed by
running the C binary, not inferred from the standard.

- **`scanf` field overflow.** glibc converts a `%d` field with `strtol`
  semantics into a `long`, saturating at `LONG_MAX`/`LONG_MIN` on overflow, and
  only then narrows to `int`. So:
  - `2147483648` becomes `INT_MIN` (`-2147483648`)
  - `4294967296` becomes `0` — as a divisor this **traps**, even though the
    written value is nonzero
  - `9223372036854775807` and any longer digit run become `-1` — as a divisor
    this turns `-2147483648 / 9999...` into the `INT_MIN / -1` trap
  - `-9223372036854775808` and anything below it become `0`

  `Scanner::scan_int` accumulates in `i128`, latches a `saturated` flag once the
  magnitude passes `i64::MAX`, clamps to `i64::MIN`/`i64::MAX`, then casts to
  `i32`.

- **Partial assignment on conversion failure.** `scanf` stops at the first
  failing directive and leaves the remaining arguments untouched, so `x` and `y`
  fall back to their initializers `1` and `1`. `quotient: 1, remainder: 0` is
  the output for empty input, whitespace-only input, and unparsable input alike.

- **Undefined division traps rather than panicking.** `die_with_sigfpe()` raises
  `SIGFPE` so the process dies by signal 8 with empty stdout, matching the
  hardware trap the C program takes. A Rust panic would have produced a message
  on stderr and a different exit status.

- **`%d` stops at the first non-digit.** `0x10` parses as `0`, `5.7` as `5`,
  `5e3` as `5`; `5-2` parses as `5` then `-2`. Whitespace, including newlines, is
  skipped freely by both the literal space in the format and by `%d` itself.

- **A lone sign is not a number.** `-`, `+`, `- 5` and `--5` are all matching
  failures, leaving the defaults in place.

- **`printf` failures are ignored.** With stdout on `/dev/full` the write fails
  and both programs still exit 0. `argv` is ignored by both, since `main()` is
  declared with no parameters.

## Result

- Both programs build with no errors (`cmake --build .`; `cargo build
  --release`).
- `cargo test` in `translation/`: 30 passed, 0 failed, 0 ignored.
- No test is disabled, skipped, or `#[ignore]`d.
- Nothing in `c_src/` was modified; only the untracked `c_src/build/` output
  directory was created.
