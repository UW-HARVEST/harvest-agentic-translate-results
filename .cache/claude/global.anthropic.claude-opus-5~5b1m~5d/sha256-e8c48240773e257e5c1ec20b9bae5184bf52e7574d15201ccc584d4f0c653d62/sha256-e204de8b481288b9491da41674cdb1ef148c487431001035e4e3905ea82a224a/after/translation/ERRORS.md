# Differential verification log

Ground truth: `c_src/src/main.c`, built with CMake (`c_src/build/driver`).
Under test: `translation/src/main.rs`, built with `cargo build --release`
(`translation/target/release/driver`).

Comparison method: both programs are spawned as subprocesses with the same
bytes on stdin; **stdout, stderr and exit status (including termination signal)**
are compared. See `translation/tests/differential.rs`.

## What the C program actually does

```
main: int x = 0; scanf("%d", &x);  // return value ignored
      if (x) good(); else bad();
      return 0;
```

Both `good()` and `bad()` zero-fill a 10-element `int` buffer from a
zero-initialized `source[10]` and then call `printIntLine(data[0])`. `data[0]`
is therefore always `0`, so **every** run prints exactly `0\n` and exits `0`.

Consequences that constrain the translation:

- The `if (x)` branch is *not observable* in the output — both branches print
  the same byte sequence. Only formatting, exit status and signal behavior can
  differ between the two programs.
- `bad()` calls `alloca(10)` (10 bytes) and then writes ten 4-byte `int`s into
  it — a 40-byte write into a 10-byte allocation. This is UB in C. Empirically
  (gcc 11.5, with and without `-O2 -fstack-protector-strong -D_FORTIFY_SOURCE=2`)
  it neither crashes nor alters the output: `0\n`, exit `0`. The Rust side
  reproduces the *observable* behavior with a correctly sized safe array rather
  than reproducing the UB.
- `scanf`'s return value is discarded, so on EOF or a matching failure `x` keeps
  its initializer `0` and `bad()` runs.
- `printLine` is defined but never called in C; the Rust translation keeps it
  behind a dead `if false` so it is likewise never reached.

## Mismatches found and fixed

### 1. Broken stdout pipe: Rust panicked where C died silently from SIGPIPE

**Status: found, fixed.**

Reproduction — give the child a pipe whose read end is already closed, so the
first write to stdout fails with `EPIPE`:

```
C:    returncode = -13 (killed by SIGPIPE), stderr = b''
Rust: returncode = -6  (SIGABRT),           stderr = b"\nthread 'main' panicked at
                                                       .../stdio.rs: failed printing
                                                       to stdout: Broken pipe (os error 32)..."
```

This differs on **both** stderr (Rust emitted a panic message; C emitted
nothing) and the exit status/signal.

**Cause.** The Rust standard library sets `signal(SIGPIPE, SIG_IGN)` before
`main` runs. A C program keeps the default disposition. With `SIGPIPE` ignored,
the failing `write` returns `EPIPE` to `println!`, which panics; because
`Cargo.toml` sets `panic = "abort"` for the release profile, the panic became
`SIGABRT` instead of exit code 101. The C program never observes the error at
all — the kernel kills it with `SIGPIPE` inside the `write`.

**Fix.** `main` now calls `restore_default_sigpipe()`, which reinstates
`SIG_DFL` for `SIGPIPE` (`signal(13, 0)`) as the first thing it does. Both
programs now terminate by signal 13 with empty stderr.

Regression test: `stdout_write_to_broken_pipe_matches`. Verified non-vacuous —
removing the `restore_default_sigpipe()` call from a scratch copy of the crate
makes exactly that test fail.

## Behaviors checked and confirmed already correct

No mismatch was found in any of these; they are listed so the next reader knows
they were exercised rather than assumed.

| Input class | Reaches | Result |
| --- | --- | --- |
| empty input / closed stdin / `/dev/null` / stdin is a directory | `scanf` → EOF, `x` stays 0, `bad()` | identical |
| `0`, `-0`, `+0`, `0000`, 30 zeros | `x == 0`, `bad()` | identical |
| `1`, `-1`, `+1`, `7`, `123456` | `x != 0`, `good()` | identical |
| `abc`, `.5`, `!!!`, `-`, `+`, `-x`, `0x10` | `scanf` matching failure, `x` untouched | identical |
| `\n`, `\n\n\n`, spaces only, `\t3`, `\r5`, `\v9`, `\f9`, blank lines then `42` | `scanf` skips whitespace *across newlines* (unlike `fgets`) | identical |
| `2147483647`, `-2147483648`, `2147483648`, `-2147483649` | `int` boundaries | identical |
| `4294967296`, `4294967297`, `8589934592`, `-4294967296` | narrowing to `int` (low 32 bits) | identical |
| `9223372036854775807`, `-9223372036854775808`, 23/300/400-digit numbers | out-of-range conversion / saturation | identical |
| `5abc`, `1e5`, `3.9`, `--1`, `1-2` | conversion stops at the first non-digit | identical |
| `1 2 3`, value followed by more lines, 70 000 trailing bytes, 70 000 leading spaces | only one conversion; the rest of stdin is ignored | identical |
| NUL bytes, `\xff\xfe\xfd`, invalid UTF-8, all 256 byte values | non-text input | identical |
| no trailing newline (`0`, `1`, `-5`, `  8`) | EOF mid-token | identical |
| stdout closed (`>&-`), extra argv entries | — | identical |
| 25 repeated runs of each branch | flakiness from `bad()`'s 40-byte write into a 10-byte `alloca` | identical every time |

In addition, a randomized sweep of **795** inputs (random bytes, random
numeric-ish tokens, and boundary values wrapped in assorted leading/trailing
whitespace) produced **0** mismatches across stdout, stderr and exit status.

## Test-suite non-vacuity

The differential harness was validated by mutating scratch copies of the Rust
crate and confirming the suite fails:

| Mutation | Result |
| --- | --- |
| `std::process::exit(0)` → `exit(1)` | 16/16 tests fail |
| `println!` → `print!` in `print_int_line` (drops trailing `\n`) | 16/16 tests fail |
| `print_int_line(data[0])` → `print_int_line(1)` | 16/16 tests fail |
| remove `restore_default_sigpipe()` | broken-pipe test fails |

## Status

- Both programs build with no errors.
- `cargo test` in `translation/`: 17 passed, 0 failed, 0 ignored.
- No test is disabled, skipped or `#[ignore]`d.
- Nothing in `c_src/` was modified.
