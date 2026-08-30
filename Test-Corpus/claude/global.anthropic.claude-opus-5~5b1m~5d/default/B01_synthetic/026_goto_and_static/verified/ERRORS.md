# Differential verification log

C ground truth: `c_src/src/main.c` → `c_src/build/driver` (cmake + cmake --build)
Rust under test: `translation/src/main.rs` → `translation/target/release/driver`
Test suite: `translation/tests/differential.rs` (20 tests, ~1100 distinct inputs)

Every test spawns **both** binaries as subprocesses, feeds identical bytes on
stdin, and compares stdout, stderr, exit code *and* terminating signal.

## Program under test

`main` does `scanf("%d %d %d", &x, &y, &z)` where `y` is the file-scope
`static int y = 123`, then calls `multi_stage(x, z)`. Four reachable outcomes:

| condition | stdout | result |
|---|---|---|
| `x != 1` | `Error: x != 1` + `Operation failed` | 1 |
| `x == 1, y != 2` | `Error: x == 1 but y != 2` + `Operation failed` | 2 |
| `x == 1, y == 2, z != 3` | `Error: x == 1 and y == 2, but z != 3` + `Operation failed` | 3 |
| all match | `Ok!` | 0 |

followed by `Result: <result>`. `main` always `return 0`, so the exit code is 0
on every non-signal path — which is exactly why the tests must compare exit
status and stderr and not just stdout.

## Mismatches found and fixed

### 1. `SIGPIPE`: Rust aborted (signal 6) where C dies from signal 13

*Symptom.* With stdout set to a pipe whose read end is already closed:

```
C    : returncode = -13   (killed by SIGPIPE), stderr empty
Rust : returncode = -6    (SIGABRT), stderr = "thread 'main' panicked ...
                           failed printing to stdout: Broken pipe (os error 32)"
```

*Cause.* The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs. The
`write(2)` therefore returns `EPIPE` instead of killing the process, `print!`
turns that `Err` into a panic, and `panic = "abort"` (set in `Cargo.toml`)
converts the panic into `SIGABRT`. A C program keeps libc's default `SIGPIPE`
disposition and is simply killed.

*Fix.* `main` now restores the default disposition before doing anything else,
via a direct `signal(2)` declaration (no new crate dependency):

```rust
extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
unsafe { signal(SIGPIPE /* 13 */, SIG_DFL /* 0 */) };
```

*Covered by.* `closed_stdout_pipe_matches_c_sigpipe_behaviour`, which also
asserts the C side really is killed by signal 13, so the test cannot silently
degrade into comparing two clean exits.

### 2. stdout write errors: Rust panicked (exit 134) where C exits 0

*Symptom.* With stdout redirected to `/dev/full` (every `write` fails
`ENOSPC`):

```
C    : exit 0, stderr empty
Rust : exit 134, stderr = "thread 'main' panicked ...
                           failed printing to stdout: No space left on device (os error 28)"
```

*Cause.* This C program never inspects the return value of `printf`, and
`return 0` from `main` reports success even though the stdio flush failed.
Rust's `print!` panics on any write error.

*Fix.* All output now goes through a small `Out` buffer whose `flush` uses
`write_all` and **discards** the `io::Result`, matching C's indifference to
write failures. `Out` also reproduces C stdio's buffering mode: line buffered
when `isatty(1)`, fully buffered otherwise.

*Covered by.* `stdout_write_error_is_ignored_like_c` (asserts C exits 0, then
asserts Rust matches) and `stdout_to_dev_null_matches`.

### 3. Flaky test harness (a defect in the test, not in the translation)

*Symptom.* `closed_stdout_pipe_matches_c_sigpipe_behaviour` failed in roughly
1 run in 15:

```
closed-pipe stdout diverged: C=(None, Some(13), []) Rust=(Some(0), None, [])
```

*Cause.* `fork(2)` copies the entire descriptor table. `cargo test` runs test
functions on parallel threads, so another test spawning its subprocess in the
window around the SIGPIPE test's `pipe()` call inherited a copy of that pipe's
read end. While that unrelated child lived, the pipe was not "closed", the
target child's `write` succeeded, and it exited 0 instead of dying from
`SIGPIPE`.

*Fix (in the test only — the Rust program was not touched).* Two measures:
`pipe2(O_CLOEXEC)` so no exec'd child retains either end, plus a `RwLock`
around every spawn site: ordinary differential runs take it for *read* and stay
parallel, while the two fd-sensitive tests take it for *write*, guaranteeing no
`fork` occurs while their pipes exist. `stdin_closed_behaves_like_eof` had the
mirror-image hazard (an inherited *write* end would prevent EOF and hang the
child) and is guarded the same way.

*Confirmed.* 20 consecutive `cargo test` plus 20 `cargo test --release` runs are
clean, and the previously-flaky test passes 30/30 when run in a tight loop under
saturating CPU load.

## Behaviours deliberately preserved (verified, no change needed)

These were audited as likely divergence points and confirmed identical:

- **`scanf` reads across newlines.** `%d` skips any run of whitespace,
  including `\n`, `\r`, `\t`, `\v`, `\f`. `1\n2\n3` is accepted exactly like
  `1 2 3`. Verified by `scanf_reads_across_newlines`.
- **Partial conversions leave later destinations untouched.** On input `1`,
  `y` keeps its initialiser `123`, so the run reports the *stage 2* error, not
  a parse error. On input `1 2`, `z` keeps `0` → stage 3 error. On empty input
  `x` keeps `0` → stage 1 error. The Rust `main` nests the three `scan_int`
  calls so a failure stops assigning, exactly as `scanf` does.
- **Matching failure vs. input failure.** `-`, `+`, `abc`, `!!!` all fail to
  convert and leave the destination alone; `scan_int` returns `None` for both
  EOF and a non-digit, which is indistinguishable here because `main` ignores
  `scanf`'s return value.
- **Decimal-only prefix parsing.** `0x1` scans as `0` and stops at `x`; `1.5`
  scans `1`; `1e5` scans `1`; `1,2,3` scans `1` then fails. Verified by
  `numeric_prefix_parsing`.
- **Overflow: glibc saturates then truncates.** glibc's `%d` accumulates
  through `strtol`, clamping to `LONG_MAX`/`LONG_MIN`, then truncates to `int`.
  So `99999999999999999999` yields `LONG_MAX as i32 == -1`, while
  `2147483648` (no `long` overflow) truncates to `-2147483648`, and
  `4294967297` truncates to `1`. `scan_int` reproduces both stages. Verified by
  `integer_limits_and_overflow`.
- **NUL and high bytes** terminate a conversion like any other non-digit;
  arbitrary bytes on stdin are handled without UTF-8 assumptions because the
  scanner works on `u8`. Verified by `non_ascii_and_nul_bytes`.
- **Chunked reads do not corrupt tokens.** The Rust scanner refills in
  4096-byte chunks; inputs were padded so tokens and whitespace runs straddle
  the 4096/8192 boundaries, plus a 1,000,000-digit token and 10,000 leading
  zeros. Verified by `buffer_refill_boundaries` and `very_large_inputs`.
- **`printf` formatting** — exact strings, single trailing `\n` each, no
  trailing whitespace, nothing on stderr on any input.

## Randomized confirmation

`randomized_sweep` (600 cases, fixed seed) draws from the alphabet that matters
to `%d` (digits, signs, whitespace variants, `.`, `x`, `e`, `,`, NUL, `0xff`);
`randomized_integer_triples` (400 cases) combines boundary values
(`0 1 2 3 -1 -3 123 INT_MAX INT_MIN 2147483648 4294967297 LONG_MAX`) with seven
separators. All 1000 match byte for byte.

## Status

- Both programs build with no errors.
- `cargo test` and `cargo test --release`: 20 passed, 0 failed, 0 ignored.
- No test is disabled, skipped or `#[ignore]`d.
- `c_src/` is unmodified (only the untracked `c_src/build/` output directory was
  created, by the prescribed cmake commands).
