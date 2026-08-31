# Verification log: C → Rust differential testing

Program under test: `c_src/src/main.c` (reads one integer with `scanf("%d")`,
prints `2*x + 300`).

- C binary: `c_src/build/driver`, built with
  `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  (no `CMAKE_BUILD_TYPE`, i.e. the default unoptimized flags).
- Rust binary: `translation/target/release/driver`, built with
  `cd translation && cargo build --release`.
- Test suite: `translation/tests/differential.rs`. It spawns both executables as
  subprocesses, feeds identical stdin, and compares stdout, stderr and
  termination status (exit code *or* signal). The Rust code is never linked as a
  library.

Nothing in `c_src/` was modified. The only addition there is the
`c_src/build/` directory produced by cmake.

## Mismatches found and fixed

### 1. `SIGPIPE`: Rust exited 0 where the C was killed by signal 13

**Symptom.** With stdout connected to a pipe whose reader had already closed:

| | stdout | stderr | termination |
|---|---|---|---|
| C | empty | empty | killed by signal 13 (`SIGPIPE`), shell status 141 |
| Rust (before fix) | empty | empty | exit code 0 |

Reproduction:

```sh
printf '5\n' | c_src/build/driver | true              # ${PIPESTATUS[1]} == 141
printf '5\n' | translation/target/release/driver | true # was 0
```

**Cause.** The Rust standard library installs `SIG_IGN` for `SIGPIPE` in its
pre-`main` runtime setup. A C program inherits the default disposition, so its
`write` (during the implicit `stdout` flush at exit) is turned into a fatal
signal. In Rust the same `write` instead returned `EPIPE`, which `main` discarded
along with every other write error, leaving a normal exit 0.

This is exactly the class of bug the task description warns about: stdout and
stderr were byte-identical, so a stdout-only assertion passed while the exit
status was wrong.

**Fix.** `translation/src/main.rs` now restores the default handler as the first
thing `main` does, via a direct `signal(2)` call:

```rust
fn reset_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
    unsafe { signal(SIGPIPE, SIG_DFL); }
}
```

Covered by `sigpipe_termination_matches`.

## Behaviors that already matched, and were pinned with tests

These are the places where the C does something surprising. Each was confirmed
against the real binary rather than assumed, and each now has a test so a future
edit cannot silently drift.

- **`scanf` reads across newlines.** `%d` skips *all* leading whitespace
  (`' '`, `\t`, `\n`, `\v`, `\f`, `\r`), so `"\n\n\n42"` yields 42. An `fgets`-style
  translation would stop at the first newline and print 300 instead.
  → `scanf_reads_across_newlines`, `whitespace_only_input`.

- **The return value of `scanf` is ignored.** On matching failure (`"abc"`,
  `"-"`, `".5"`) and on EOF (empty input), `x` keeps its initializer `0` and the
  program prints `300` and exits 0. There is no error path and nothing is ever
  written to stderr. → `empty_input`, `matching_failure_paths`.

- **A lone sign is a matching failure, not a zero.** `"-"`, `"+"`, `"- 5"`,
  `"--5"` all print 300. → `explicit_signs`, `matching_failure_paths`.

- **Values wider than `int` are truncated, not rejected.** glibc converts `%d`
  through a `long` and stores the low 32 bits, so `4294967296` → `0` → prints
  `300`, and `4294967301` → `5` → prints `310`.
  → `values_beyond_int_are_truncated`.

- **Values beyond `long` saturate, then truncate.** glibc clamps to
  `LONG_MAX`/`LONG_MIN` and stores the low 32 bits, so
  `99999999999999999999999` → `-1` → prints `298`, while
  `-99999999999999999999999` → `0` → prints `300`. The Rust `scanf_d` reproduces
  this with an `i64` accumulator plus a saturation flag, then `as i32`.
  → `values_beyond_long_saturate`, `very_long_digit_runs` (2000- and
  10000-digit runs).

- **`2*x + 300` wraps.** Signed overflow is UB in C but the emitted code wraps,
  so `1073741674` prints `-2147483648`. The Rust uses `wrapping_mul` /
  `wrapping_add`; note that plain `*`/`+` would panic in a debug build, so this
  matters for `cargo test` as well as `cargo build --release`.
  → `arithmetic_overflow_wraps`, `int_boundaries`.

- **`%d` is strictly base 10.** `"010"` is ten, not eight; `"0x1f"` converts as
  `0` and leaves `x1f` unread. → `leading_zeros_are_decimal`.

- **Only the first number is consumed.** `"1 2 3"` prints `302`; the unread tail
  is simply discarded at exit, even when it is a megabyte long.
  → `only_first_number_is_read`, `large_unread_tail`.

- **NUL bytes are ordinary non-digit bytes.** They neither terminate nor count as
  whitespace, so `"\0" + "5"` is a matching failure. → `embedded_nul_bytes`.

- **`printf("%d\n")` has no padding and exactly one trailing newline**, for every
  magnitude and for negative results. → `formatting_across_magnitudes`,
  `output_sign_transitions` (`-150` → `0`, `-151` → `-2`).

- **`argv` is never read**, so extra arguments change nothing.
  → `arguments_are_ignored`.

- **Unreadable stdin behaves like EOF.** Closed fd 0, `/dev/null`, and a
  directory on stdin all print `300` and exit 0.
  → `stdin_closed`, `stdin_from_dev_null`, `stdin_is_a_directory`.

- **A failed stdout write that is not a broken pipe is swallowed.** With stdout
  on `/dev/full` the flush fails with `ENOSPC` and the C still exits 0 with empty
  stderr; the Rust must not report the error. → `stdout_write_error_is_ignored`.

## Coverage beyond the enumerated cases

Two brute-force layers were run to look for input classes the hand-enumeration
missed:

- `exhaustive_short_inputs` (in the suite): every byte string of length 1–3 over
  the alphabet the conversion distinguishes — `' '`, `\n`, `\t`, `-`, `+`, `0`,
  `9`, `a`, `.`, `\0` — which pins the ordering of the whitespace skip, the sign
  check and the digit check.
- An out-of-band sweep (not part of the suite): all 4096 length-4 strings over
  the same alphabet, 3000 random byte strings, 3000 random numeric strings, and
  6000 random integers spanning 1 to 30 decimal digits in both signs.
  **16096 inputs, 0 mismatches.**

## Result

`cargo test` passes in `translation/` with 26 tests, none `#[ignore]`d, skipped
or disabled, on both the debug and release profiles.
