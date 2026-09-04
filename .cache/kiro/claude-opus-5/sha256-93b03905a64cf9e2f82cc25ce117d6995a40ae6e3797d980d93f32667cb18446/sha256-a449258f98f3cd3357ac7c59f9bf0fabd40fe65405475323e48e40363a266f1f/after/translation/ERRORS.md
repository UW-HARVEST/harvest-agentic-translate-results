# Differential verification of `c_src/src/main.c` vs. this Rust crate

The C program is the ground truth. This file records every behavioural
difference found between the two binaries, what caused it, and how the Rust
side was changed. The C sources were not modified (verified: only
`c_src/build/`, the out-of-source CMake build directory, was added).

## How it was verified

* C reference: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
* Rust: `cd translation && cargo build --release` → `translation/target/release/driver`
* `translation/tests/differential.rs` spawns **both binaries** as subprocesses
  with identical stdin and asserts stdout, stderr and exit status (exit code
  *and* terminating signal) are byte-for-byte equal. Nothing is loaded as a
  library.
* In addition, ~20k randomly generated inputs (mixtures of decimal, hex,
  `inf`/`nan`, garbage bytes, over-length lines and empty lines) were compared
  the same way during development. All matched.

## Mismatches found

### 1. `SIGPIPE`: Rust exited 134 with a panic message where C exited 141 silently

**Symptom** — with the write end of stdout attached to an already-closed reader:

```
$ ./c_src/build/driver < in | true            ; # C
exit status 141 (killed by SIGPIPE), stderr empty
$ ./translation/target/release/driver < in | true   ; # Rust, before the fix
exit status 134 (SIGABRT), stderr:
thread 'main' panicked at library/std/src/io/stdio.rs: failed printing to stdout: Broken pipe (os error 32)
```

**Cause** — Rust's runtime sets `SIGPIPE` to `SIG_IGN` before entering `main`,
so a failed write surfaces as `EPIPE`; `println!` panics on write errors, and
`panic = "abort"` turns that into `SIGABRT`. A C program inherits the default
disposition and is killed by the signal instead, and C code that ignores
`printf`'s return value never notices a write error at all.

**Fix** — `cruntime::reset_sigpipe()` restores `SIG_DFL` for `SIGPIPE` as the
first statement of `main`, and the flush path discards write errors the way the
C code ignores `printf`'s return value.

Covered by `broken_pipe_exit_status_matches` and
`broken_pipe_produces_no_rust_panic_message`.

### 2. Output buffering discipline differed from C's stdio

**Symptom** — a partially-consuming reader observed different behaviour, e.g.
`driver | head -1`: C finished with status 0, Rust was killed at status 141.

**Cause** — Rust's `println!` writes through a `LineWriter`, which flushes on
every newline regardless of what stdout is. glibc chooses at first use: line
buffered only when stdout is a terminal, otherwise fully buffered (`BUFSIZ`),
so this program's ~130 bytes of output reach a pipe in a single write at exit.
With per-line writes, Rust hit the closed pipe on its second line while C never
performed a failing write at all.

**Fix** — `cruntime` emulates the C stream: output accumulates in a buffer that
is flushed when it reaches `BUFSIZ` or when `main` exits, and only flushes per
line when `isatty(1)`. Verified against the C binary both through a pipe and
through a pty.

Covered by `broken_pipe_produces_no_rust_panic_message` (and the pty check
described above, which is not part of the test suite because it needs a tty).

## Behaviours deliberately preserved, not "fixed"

These are not bugs in the translation; they are the C program's behaviour and
the Rust code reproduces them on purpose.

* **Divide by zero in `bad()`.** `100.0 / 0.0F` is `+inf`, and `(int)` of an
  out-of-range double is undefined behaviour in C. The reference build on
  x86-64 emits `cvttsd2si`, which yields the "integer indefinite" value
  `INT_MIN`, so the program prints `-2147483648`. `cruntime::f64_to_int`
  returns `i32::MIN` for NaN and for anything outside `[-2^31, 2^31)`, matching
  the C binary for every input tried, including the exact boundary
  `100.0 / 0x1.9p-25 == 2147483648.0`.
* **`fgets`, not `scanf`.** Reading stops at a newline and never crosses one;
  at most `CHAR_ARRAY_SIZE - 1 == 19` bytes are stored per call, and the
  remainder of an over-long line is picked up by the *next* call. So
  `1\n2\n3\n` gives `goodB2G()` the value `1` and `bad()` the value `2`, and a
  25-character line is split between the two.
* **Two independent reads.** `goodB2G()` reads first, then `bad()`. One line of
  input therefore prints `fgets() failed.` from `bad()` only.
* **`fabs(data) > 0.000001` is a strict comparison**, so exactly `0.000001`
  takes the "This would result in a divide by zero" branch. `NaN` also takes
  that branch, because every comparison with `NaN` is false.
* **`atof` == `strtod(s, NULL)`**: leading whitespace and sign, hex (`0x1.8p3`)
  and `inf`/`nan` forms are accepted, the longest valid prefix wins
  (`12abc` → `12`), incomplete exponents are not consumed (`5e` → `5`), and an
  unconvertible string yields `0.0` rather than an error.
* **`(float)atof(...)` truncates to single precision** before the division, so
  `1e-40` becomes a subnormal float and `1e-300` becomes `0.0F` — which is why
  those inputs reach the divide-by-zero paths.
* **A NUL byte ends the C string** even though `fgets` stored bytes past it, so
  `1\x002` converts as `1`.
* **`printLine(NULL)` is unreachable** in this program: every call site passes
  a literal. The `Option`-taking signature is kept for fidelity but only the
  `Some` arm is exercised, so there is no observable behaviour to compare.
* **`argc`/`argv` are ignored**, so arguments cannot change the output.

## Status

`cargo test` in `translation/` passes (17 tests, none ignored or skipped) for
both the default and `--release` profiles, and every enumerated input produces
identical stdout, stderr and exit status.
