# Differential verification log

Reference: `c_src/src/main.c` (CMake target `driver`, built with no
`CMAKE_BUILD_TYPE`, i.e. no optimization flags).

Commands used:

| program | command |
|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `c_src/build/driver <initial_value> <iterations>` |
| Rust | `cd translation && cargo build --release` → `translation/target/release/driver <initial_value> <iterations>` |

Tests live in `translation/tests/differential.rs`. They spawn both binaries as
subprocesses with identical `argv` and compare stdout, stderr and exit status
(including death-by-signal) for every input class.

## Mismatches found and fixed

### 1. Exit status on a closed stdout pipe (SIGPIPE)

* **Symptom** — `driver 5 5000000 | head -1` gave shell status `141`
  (killed by `SIGPIPE`) for the C program and `0` for the Rust program. Both
  produced the same bytes on the lines that were read, so a stdout-only test
  passed while the status silently disagreed.
* **Cause** — the Rust standard library sets `SIGPIPE` to `SIG_IGN` before
  `main` runs. Writes to a broken pipe therefore return `EPIPE` instead of
  raising the signal, and the translation was discarding write errors with
  `let _ = ...`, so it ran to completion and exited `0`. The C program has the
  default disposition and is killed by the signal.
* **Fix** — `restore_default_sigpipe()` in `translation/src/main.rs` calls
  `signal(SIGPIPE, SIG_DFL)` as the first statement of `main` (declared via
  `extern "C"`, so no new dependency). Regression test:
  `closed_stdout_pipe_terminates_the_same_way`.

### 2. stdout buffering discipline

* **Symptom** — no output difference on its own, but the Rust program wrote
  through a `LineWriter` (one `write` syscall per line, regardless of whether
  stdout is a terminal) while C stdio fully buffers a non-tty stdout. Combined
  with fix 1 this changes how much output is flushed before a broken pipe kills
  the process.
* **Fix** — stdout is wrapped in a `BufWriter` so the write pattern matches C
  stdio on the pipes and files the programs are actually compared over.

## Behaviors deliberately preserved (verified equal, no change needed)

These all looked like candidate bugs and were confirmed to already match; they
are pinned by tests so a future edit cannot regress them.

* **Error messages go to stdout, not stderr.** The C code uses `printf` for all
  three error messages, so stderr is empty on every path. Exit code is `1`.
  (`nothing_is_written_to_stderr`, `wrong_argument_count_is_rejected`)
* **Validation order.** `argc` is checked before either `strtol`, and `argv[1]`
  before `argv[2]`, so an invalid first argument masks an invalid second one.
  (`first_argument_checked_before_second`)
* **`strtol` acceptance is "at least one digit consumed".** The C code only
  tests `end == argv[i]`, so trailing garbage is accepted: `12abc` → `12`,
  `0x10` → `0`, `3.9` → `3`, `5 5` → `5`. Leading `isspace` runs and one
  optional sign are consumed; a sign with no digit following is a parse
  failure. (`partial_parses_are_accepted`, `first_argument_must_parse`,
  `second_argument_must_parse`)
* **`long` → `int` narrowing.** Both operands are `int` but `strtol` returns
  `long`, so `4294967296` becomes `0` and `4294967295` becomes `-1`. The
  iteration count truncates the same way, which is how `4294967300` yields
  exactly 4 lines. (`long_to_int_truncation_of_initial_value`,
  `long_to_int_truncation_of_iteration_count`)
* **`strtol` range saturation.** Out-of-range input saturates to `LONG_MAX` /
  `LONG_MIN` before narrowing, so `99999999999999999999999999` → `-1` and
  `-99999999999999999999999999` → `0`. All digits are still consumed, so this
  is a success, not a parse error. (`strtol_range_saturation`)
* **Signed overflow wraps.** The accumulator overflows `int` once the doubling
  starts; the reference build is unoptimized, so it wraps two's-complement. The
  translation uses `wrapping_add` to match. (`int_boundary_initial_values`)
* **The self-aliasing quirk.** Once `static_alias` returns `&inner`, the next
  call receives `outer == &inner`, so `*outer >= inner` compares the variable
  with itself and is always true: `inner += *outer` becomes `inner += inner`.
  The function can never leave that state, so the sum doubles forever. The
  translation models the returned pointer as a tag (`Ref::Outer` / `Ref::Inner`)
  over two plain locals, which is observationally identical in safe Rust.
  (`then_branch_then_self_aliasing`)
* **The `else` branch walk.** For `initial_value < 1` the automatic variable is
  incremented by `inner` (still `1`) once per iteration until it reaches `1`,
  at which point control flips to the aliased state. Verified for every walk
  length over `-40..=40`. (`else_branch_walks_up_to_zero_then_aliases`,
  `boundary_initial_values_scan`)
* **Non-UTF-8 `argv`.** Arguments are read with `args_os()` and handled as
  bytes, so invalid UTF-8 reaches the parser instead of panicking.
  (`non_utf8_arguments`)
* **No trailing or leading extra output.** `printf("%d\n", ...)` per iteration
  and nothing else; zero and negative iteration counts print nothing at all and
  exit `0`. (`zero_iterations_produces_no_output`,
  `negative_iterations_produce_no_output`)

## Coverage of the C control flow

Every branch in `main` and `static_alias` is reached by at least one test:

| site in `c_src/src/main.c` | test |
|---|---|
| `if (argc != 3)` taken (argc 0,1,3,4) | `wrong_argument_count_is_rejected` |
| `if (argc != 3)` not taken | all others |
| `if (end == argv[1])` taken | `first_argument_must_parse` |
| `if (end == argv[2])` taken | `second_argument_must_parse` |
| both `strtol` checks not taken | `partial_parses_are_accepted` |
| loop body never entered | `zero_iterations_produces_no_output`, `negative_iterations_produce_no_output` |
| loop body once | `single_iteration` |
| `*outer >= inner` true, `outer != &inner` | `then_branch_then_self_aliasing` |
| `*outer >= inner` false (`else`) | `else_branch_walks_up_to_zero_then_aliases` |
| `*outer >= inner` with `outer == &inner` (aliased) | `then_branch_then_self_aliasing`, `int_boundary_initial_values` |
| `return 0` at the end | every successful run |

## Test-sensitivity check

To confirm the suite is not vacuous, seven faults were injected into the Rust
program one at a time; each was caught and the source was restored:

| injected fault | tests failed |
|---|---|
| `exit(0)` instead of `exit(1)` on the argc error | 1 |
| reworded argc error message | 1 |
| `restore_default_sigpipe()` removed | 1 |
| `saturating_add` instead of `wrapping_add` | 8 |
| `strtol` range clamp returning `0` instead of `LONG_MAX` | 2 |
| first-argument error written to stderr | 5 |
| aliased `inner += inner` changed to `inner += 1` | 11 |

## Known limitation

`argc == 0` is not covered: `std::process::Command` always passes `argv[0]`, so
the case cannot be produced without a raw `execve`. Both programs treat it as
`argc != 3` and take the same error path.
