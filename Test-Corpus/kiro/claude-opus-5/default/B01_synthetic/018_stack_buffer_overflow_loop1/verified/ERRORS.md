# ERRORS.md — differential findings for the C → Rust translation

Reference: `c_src/src/main.c` (built with CMake to `c_src/build/driver`)
Translation: `translation/src/main.rs` (built to `translation/target/release/driver`)
Tests: `translation/tests/differential.rs` — 20 tests, each running **both**
binaries as subprocesses and comparing stdout, stderr and exit status.

Commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
cd translation && cargo test                                            # differential suite
```

## What the C program can actually do

`main()` is the whole reachable surface:

```c
int x = 0;
scanf("%d", &x);
if (x) good(); else bad();
return 0;
```

Both `good()` and `bad()` copy a zero-initialised `int source[10]` into a
buffer and print element 0, so **stdout is `"0\n"` for every input**, stderr is
always empty, and the exit status is always 0 (absent a signal). The `scanf`
result is discarded and `x` is pre-initialised to 0, so every parse failure
lands on the `bad()` branch.

This makes stdout a weak oracle: it cannot distinguish the two branches. The
mismatches below were therefore found on the exit-status and semantics axes,
not the stdout axis.

## Mismatch 1 — exit status on a broken stdout pipe (real, fixed)

**Symptom.** With stdout connected to a pipe that has no reader:

| | termination |
|---|---|
| C | killed by signal 13 (`SIGPIPE`) |
| Rust (before fix) | exited 0 |

**Cause.** The Rust runtime installs `SIG_IGN` for `SIGPIPE` before `main`
runs, so a failing write returns `EPIPE` instead of raising the signal. The C
program inherits the default disposition and is killed. Compounding it, the
translation discarded write errors (`let _ = write!(...)`), so even the `EPIPE`
was invisible and the process fell through to `return 0`.

**Fix.** `restore_default_sigpipe()` in `translation/src/main.rs` resets
`SIGPIPE` to `SIG_DFL` as the first statement of `main`, via a direct `extern
"C" { fn signal(...) }` declaration so the crate stays dependency-free. Both
programs are now killed by signal 13.

**Regression test.** `broken_stdout_pipe_terminates_identically` — borrows a
pipe from a helper `cat`, kills the helper so the read end is gone, then hands
the write end to each program as stdout and compares how each terminated. Note
that a stdout-only comparison can never catch this class of bug, which is why
the harness asserts on all three of stdout, stderr and status.

## Mismatch 2 — `scanf` return value for sign-then-EOF (latent, fixed)

**Symptom.** For input `"-"` or `"+"` (a sign immediately followed by EOF), the
translation's `scanf_i32` returned `-1` (`EOF`, i.e. an *input* failure). glibc
returns `0` — a *matching* failure — because characters were already consumed,
so it is not an input failure per the C standard.

Verified against glibc with a standalone probe (`ret` is the `scanf` return,
`x` pre-set to `-12345` to expose whether it is written):

```
[-]   -> ret=0 x=-12345
[+]   -> ret=0 x=-12345
[abc] -> ret=0 x=-12345
[]    -> ret=-1 x=-12345      <- genuine input failure
```

**Not observable in this program**, since `main` discards the `scanf` result
and `x` is left untouched either way, so both values route to `bad()`. Fixed
anyway, because a wrong-by-luck translation is a trap for the next change.

**Fix.** The sign-then-EOF arm of `scanf_i32` now returns `0`.

## Verified-correct behaviors (checked, no change needed)

Confirmed against glibc with the same probe; the Rust `scanf_i32` reproduces
each stored value exactly.

- **Whitespace skipping crosses newlines.** `%d` skips space, tab, newline,
  vertical tab, form feed and carriage return before converting — unlike
  `fgets`. `"\n\n\n7"` parses as 7 and reaches `good()`.
  Tests: `scanf_skips_leading_whitespace_across_newlines`.
- **Overflow truncates a saturated `long`.** glibc accumulates `%d` into a
  `long`, saturates at `LONG_MAX`/`LONG_MIN` on range error, then stores the
  low 32 bits. This produces distinctly un-obvious values, all matched:

  | input | stored `int` | branch |
  |---|---|---|
  | `2147483648` | `-2147483648` | `good()` |
  | `4294967296` | `0` | **`bad()`** |
  | `4294967297` | `1` | `good()` |
  | `-2147483649` | `2147483647` | `good()` |
  | `99999999999999999999999` | `-1` | `good()` |
  | `-99999999999999999999999` | `0` | **`bad()`** |
  | `9223372036854775808` | `-1` | `good()` |
  | `-9223372036854775808` | `0` | **`bad()`** |

  The `4294967296` and `-99999999999999999999999` rows flip the branch relative
  to a naive saturate-to-`INT_MAX` reading, so they are asserted rather than
  assumed. Test: `single_item_and_maximum_values`.
- **Conversion stops at the first non-digit.** `0x10` yields 0 (stops at `x`,
  reaching `bad()`), `3.7` yields 3, `1abc` yields 1.
  Test: `scanf_stops_at_first_non_digit`.
- **Matching failure leaves `x` at its initial 0.** `abc`, `.`, `-`, `+`,
  `--5`, `- 5` all reach `bad()`. Test:
  `scanf_matching_failures_leave_x_at_zero`.
- **Only one conversion is consumed.** `main` calls `scanf` once; trailing
  input is never read, and the unread remainder does not alter output.
  Test: `only_the_first_conversion_is_consumed`.
- **Empty and whitespace-only input.** Both give `scanf` EOF, leaving `x == 0`
  and reaching `bad()`. Tests: `empty_input_takes_the_bad_branch`,
  `whitespace_only_input_is_eof_for_scanf`, `stdin_closed_immediately`.
- **stdout on a closed descriptor (`>&-`).** Writes fail with `EBADF`, which
  raises no signal; both programs ignore the error and exit 0. Distinct from
  Mismatch 1. Test: `stdout_redirected_to_a_closed_descriptor`.
- **`argv` is ignored.** `main()` takes no parameters.
  Test: `command_line_arguments_are_ignored`.
- **Non-UTF-8, NUL and high bytes on stdin** are handled identically; the
  harness pipes raw `&[u8]`, never `String`. Test: `binary_and_nul_bytes`.
- **Inputs far larger than any buffer** (100,000-byte digit, zero, whitespace
  and alpha runs). Test: `very_long_inputs`.

## Deliberate non-reproduction: the `alloca` overflow in `bad()`

```c
data = (int *)alloca(10);          /* 10 bytes */
for (i = 0; i < 10; i++) data[i] = source[i];   /* writes 40 bytes */
```

`bad()` allocates 10 bytes and writes 40 — a stack buffer overflow, and
undefined behavior in C (this is the CWE demonstration the file exists for).
`good()` is the corrected form, `alloca(10 * sizeof(int))`.

The translation uses a correctly sized buffer in both functions and does **not**
recreate the out-of-bounds write. Rationale: the only thing the overflow exposes
is `data[0]`, which is 0 in both functions because `source` is zero-initialised.
UB has no defined behavior to match, so there is nothing to be faithful *to* —
the choice is between reproducing one incidental observation of it or matching
the observable contract. Observationally the C program prints `0\n` here, and
`repeated_runs_are_deterministic` confirms this over 25 runs of each branch,
so the C side is stable and the two agree.

The one respect in which the C and Rust `bad()` genuinely differ is memory
safety, and that is not observable through stdout, stderr or exit status. If the
C were rebuilt with a hardening flag that turns the overflow into a trap
(`-fsanitize=address`, or a stack protector that this overflow happens to trip),
the C would abort and the Rust would not. The suite builds the C program with
the unmodified `c_src/CMakeLists.txt`, which sets no such flags, so this does
not arise as configured — but it is the known limit of the equivalence claim
rather than something the tests establish.

## Unreachable code

`printLine(const char *)`, including its `line != NULL` branch, is never called
from `main`. No input can reach it, so no test covers it; it is retained in the
Rust as `print_line` under `#[allow(dead_code)]` for structural fidelity with
the original translation unit.

## Status

- Both programs build with no errors.
- `cargo test` passes: 20 tests, 0 failed, 0 ignored — in both the debug and
  release profiles.
- No test is disabled, skipped or `#[ignore]`d.
- Nothing in `c_src/` was modified.
