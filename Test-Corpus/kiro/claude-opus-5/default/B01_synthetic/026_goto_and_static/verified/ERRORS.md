# Differential verification findings

Ground truth: `c_src/src/main.c`, built with CMake to `c_src/build/driver`.
Candidate: `translation/src/main.rs`, built with `cargo build --release` to
`translation/target/release/driver`.

Both programs are compared by execution only — same bytes on stdin, same argv —
diffing stdout, stderr, exit code and terminating signal. The tests live in
`tests/differential.rs` (input-space coverage) and `tests/process_io.rs`
(process-level I/O edge cases), sharing the subprocess harness in
`tests/common/mod.rs`.

## Mismatches found

### 1. `SIGPIPE` disposition: C is killed by the signal, Rust exited 0

**Symptom.** With stdout connected to a pipe whose read end had already been
closed, the two programs disagreed on how they terminated:

| | exit code | signal |
|---|---|---|
| C | *(none)* | 13 (`SIGPIPE`) |
| Rust (before fix) | 0 | *(none)* |

Reproduced by `process_io::reader_hangup_on_stdout`, which spawns the program
with a piped stdout, drops the read end, then writes `1 2 3\n` to its stdin.

**Cause.** The Rust standard library installs `SIG_IGN` for `SIGPIPE` during
runtime setup, before `main` is entered. With the signal ignored, the failing
`write` returns `EPIPE`, and because the translation deliberately discards write
errors (matching C's habit of ignoring `printf` return values) the process ran
to completion and exited 0. The C program never touches the disposition, so it
keeps the default action and is killed by the signal instead. This is purely a
runtime-environment difference; no logic in the translation was wrong.

Note that this was invisible to every stdout/stderr comparison — the signalled C
process produces no output at all, and so did the Rust process, since its single
buffered write also failed. Only the exit status distinguished them, which is
exactly the failure mode the task warns about.

**Fix.** `translation/src/main.rs` now resets the disposition to `SIG_DFL` as the
first statement of `main`, via a locally declared `signal` binding so no new
crate dependency is introduced:

```rust
#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
    unsafe { signal(SIGPIPE, SIG_DFL); }
}
```

Both programs now die with signal 13 and no exit code in this scenario.

## Behaviours checked and confirmed already correct

These were candidate divergences worth verifying; the translation matched C on
all of them, so no change was needed. They are recorded because "we looked" is
the useful information.

- **`static int y = 123` is observable.** `y` is the global that `scanf` writes
  into. When fewer than two conversions succeed, `multi_stage` reads the
  initialiser `123` and reports stage 2. Input `1` alone exercises this.
- **Partial `scanf` assignment.** A failing `%d` leaves its target untouched and
  aborts the rest of the format string. The four cases — 0, 1, 2 and 3
  successful conversions — map to results 1, 2, 3 and 0 respectively, since `x`
  and `z` start at 0 and `y` at 123.
- **`%d` skips whitespace across lines.** `\n`, `\t`, `\r`, `\v`, `\f` and runs
  of 100 000 spaces are all consumed transparently; `1\n2\n3` behaves the same
  as `1 2 3`.
- **Sign handling and matching failures.** `+1` converts; a lone `-`, `- 1`,
  `--1`, `++1`, `+-1`, `.1`, `abc` and `e` are all matching failures that assign
  nothing.
- **Base 10 only.** `0x10` converts as `0` and leaves `x` at the `x`, which then
  fails the next conversion. `010` is ten, not eight.
- **Truncation to `int` is observable and reproduced.** `4294967297` truncates
  to `1` and *passes* stage 1; `4294967298` for `y` passes stage 2;
  `4294967299` for `z` passes stage 3. An input of
  `4294967297 4294967298 4294967299` prints `Ok!` in both programs.
- **glibc saturates before truncating.** The accumulator follows `strtol`
  semantics: values beyond `long` clamp to `LONG_MAX`/`LONG_MIN` and only then
  truncate to `int`, so `LONG_MAX as i32 == -1`. This is distinguishable from a
  wrapping accumulator: `18446744073709551618` (2^64 + 2) would truncate to `2`
  if the accumulator wrapped mod 2^64, but both programs report `y != 2`. The
  translation's `saturating_mul`/`saturating_add` on `u64` followed by a clamp to
  `i64` range reproduces this, verified up to 100 000-digit inputs.
- **`argv` is ignored.** The C `main` takes no parameters, so arguments never
  affect behaviour.
- **Write errors are swallowed.** With stdout on `/dev/full` (every write fails
  with `ENOSPC`) both programs still exit 0 and write nothing to stderr, because
  C ignores `printf`'s return and the translation ignores its `write!` results.
- **A closed stdin descriptor behaves like EOF.** With fd 0 closed outright so
  reads fail with `EBADF`, both programs treat it as zero successful conversions
  and print the stage 1 error.
- **Raw bytes.** NUL bytes, bytes `0x80`–`0xff`, full-width Unicode digits and a
  payload containing all 256 byte values all produce identical output; the
  translation reads stdin as bytes rather than as UTF-8.
- **Exit status is otherwise always 0**, since C `main` ends in `return 0` on
  every path.

## Coverage of the C source

Every branch in `c_src/src/main.c` is reached by at least one test:

| C construct | reached by |
|---|---|
| `if (x != 1)` → result 1 | `stage1_x_not_one`, `zero_conversions_*` |
| `if (y != 2)` → result 2 | `stage2_y_not_two`, `one_conversion_only_x` |
| `if (z != 3)` → result 3 | `stage3_z_not_three`, `two_conversions_only_x_and_y` |
| `printf("Ok!\n"); return result;` | `stage_all_pass`, `three_conversions_happy_path` |
| `fail:` label (all three `goto`s) | all three stage tests |
| `scanf` with 0/1/2/3 conversions | the four `*_conversions_*` tests |
| `printf("Result: %d\n", ...)` | every test (byte-compared) |
| `return 0` from `main` | every test (exit status compared) |

In addition, `exhaustive_small_value_grid` covers all 343 combinations of
`x, y, z` in `-2..=4`, and two seeded pseudo-random sweeps cover 600 token /
separator combinations and 300 random byte strings. A separate 4 000-case
ad-hoc fuzz run over the same alphabet found no further mismatches.

## Status

- Both programs build with no errors and no warnings.
- `cargo test` passes: 28 tests, 0 failed, 0 ignored.
- No test is disabled, skipped or `#[ignore]`d.
- Nothing under `c_src/` was modified; only the CMake-owned out-of-source
  `c_src/build/` directory was created.
