# Mismatches found while differential-testing the translation

The C program (`c_src/src/main.c`) is the ground truth. Each entry below is a
divergence that was observed by running both binaries on the same input and
comparing stdout, stderr and exit status.

Reference environment: GCC 11.5.0 / glibc, x86-64 Linux, `cmake` defaults (no
optimization flags, as `c_src/CMakeLists.txt` specifies).

Commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
cd translation && cargo test                                            # differential suite
```

---

## 1. Exit status diverged when stdout is a pipe with a closed read end

**Symptom**

```
$ echo 1 | ./c_src/build/driver           > >(exec 0<&-; :) ; echo $?
141
$ echo 1 | ./translation/target/release/driver > >(exec 0<&-; :) ; echo $?
0
```

stdout and stderr were both empty and identical; only the exit status differed.
`141` is `128 + 13`, i.e. killed by `SIGPIPE`.

**Cause**

The Rust standard library sets `SIGPIPE` to `SIG_IGN` during runtime setup,
before `main` is entered. A write to a broken pipe therefore returns `EPIPE`
instead of raising a signal, and because `print_line` discards its write result
the process went on to exit 0. The C program keeps the default disposition and
is killed by signal 13 — for `x != 0` at the `printf` in `printLine`, and for
`x == 0` at the stdout flush that glibc performs while exiting.

**Fix**

`reset_sigpipe()` in `src/main.rs` calls `signal(SIGPIPE, SIG_DFL)` as the first
statement of `main`, restoring C's disposition. `libc` is already linked on the
`*-unknown-linux-gnu` targets, so the symbol is declared in an `extern "C"`
block rather than pulling in a dependency.

**Regression test**

`closed_stdout_pipe_kills_both_the_same_way`. The child is spawned with all
three streams piped and the read end of its stdout is dropped *while the child
is still blocked in `scanf`*, before stdin has been written. The failure is
therefore deterministic rather than a race.

---

## 2. Unconsumed stdin was not handed back, so a later reader of the same descriptor saw different bytes

**Symptom**

```
$ printf '1 REST-OF-FILE\nsecond line\n' > f
$ { ./c_src/build/driver; cat; } < f
string
 REST-OF-FILE
second line
$ { ./translation/target/release/driver; cat; } < f
string
```

With a 20001-byte file, `{ driver >/dev/null; wc -c; } < f` reported 20000 bytes
left for C and 11809 (= 20001 − 8192) for Rust.

**Cause**

The translation read stdin through `std::io::Stdin`, which wraps the descriptor
in an 8 KiB `BufReader`. Two separate errors followed from that:

* **No offset restoration.** glibc, while tearing the stream down at exit, seeks
  a seekable descriptor back by the part of the `FILE` buffer that was never
  consumed, leaving the offset exactly past the last byte the conversion used.
  Rust's `BufReader` just drops its buffer, permanently losing those bytes.
* **Wrong buffer size.** glibc sizes the buffer from `st_blksize`, which is 4096
  for both regular files and pipes on Linux; Rust's `BufReader` uses 8192. On a
  *pipe* nothing can be seeked back, so the buffer size alone decides how much a
  single `scanf` removes from the stream. For a 60001-byte pipe the leftover was
  55905 bytes (60001 − 4096) for C and 51809 (60001 − 8192) for Rust.

**Fix**

`CStdin` in `src/main.rs` was rewritten to model glibc's `FILE` directly: a
4096-byte buffer filled by `read(2)` on fd 0, a read cursor, and
`restore_offset()`, which `main` calls at exit to `lseek(0, -(unconsumed),
SEEK_CUR)`. The `lseek` fails with `ESPIPE` on a pipe and the failure is ignored,
which is also the net effect in glibc.

`ungetc` is now modelled by stepping the read cursor back one byte instead of
holding the byte in a side channel, so a pushed-back byte counts as unconsumed
and is included in the offset that gets handed back. This matters: for
`aREST-OF-FILE` the C program leaves the `a` readable, and for `+aREST-OF-FILE`
it leaves `aREST-OF-FILE` — the sign is consumed, only the single byte that
stopped the conversion is pushed back.

**Regression tests**

`regular_file_stdin_leaves_the_offset_where_c_leaves_it` (14 cases, including
every pushback variant above) and `pipe_stdin_swallows_exactly_one_block`.

---

## Behavior that looks wrong but is correct, and was preserved

These were checked against the C binary and deliberately left as they are.

* **`bad()` prints a bare newline.** `bad()` reads an uninitialized `char *data`,
  which is undefined behavior. In the reference build the stack slot holds
  residue from the preceding `scanf` frame: a non-`NULL` pointer whose first byte
  is `0`. `printLine` therefore takes the non-`NULL` branch and `printf("%s\n",
  data)` emits a single `\n`. Confirmed for every `x == 0` input tried, including
  1440 fuzz cases. The translation reproduces the *observed output* by passing
  `Some("")`; it does not read an actually-uninitialized value, which would be UB
  in Rust too. The defect is preserved, not repaired — `bad()` still fails to
  print anything meaningful.
* **Overflow saturates at `long`, then truncates to `int`.** glibc's `%d`
  accumulates at `long` width, clamps to `LONG_MAX`/`LONG_MIN`, and stores
  through an `int *`. The visible consequences: `4294967296` stores `0` and takes
  the `bad()` branch, `2147483648` stores `INT_MIN` and takes `good()`,
  `18446744073709551617` saturates to `LONG_MAX` and stores `-1` (`good()`),
  `-9223372036854775809` saturates to `LONG_MIN` and stores `0` (`bad()`).
* **`scanf` reads across newlines.** `%d` skips leading whitespace including
  `\n`, so `"\n\n\n5"` converts to 5. An `fgets`-based translation would report
  a matching failure here instead.
* **`scanf`'s return value is discarded.** `main` ignores it, so a matching
  failure and an EOF are indistinguishable: `x` keeps its initializer `0` in both
  cases and `bad()` runs. `scanf_d` still returns the value for fidelity.
* **`argv` is ignored.** `main()` is declared with no parameters; extra
  arguments change nothing.
* **No error path writes to stderr and the exit status is always 0.** Absent the
  two signal/stream issues above, stderr is empty and the status is 0 for every
  input.

---

## Verification performed

* 20 tests in `translation/tests/differential.rs`, none `#[ignore]`d, skipped or
  disabled. Every test spawns both binaries as subprocesses and compares stdout
  bytes, stderr bytes and exit status (including termination by signal). The
  translation is never loaded as a library.
* Input classes covered: empty input; a single item; `x == 0` and `x != 0`; all
  six C whitespace characters as leading padding, including across newlines;
  matching failure; conversion stopping at a non-digit; every sign form including
  a sign with no digit after it; `INT`/`LONG`/`unsigned` boundaries and
  saturation; NUL, high-bit and multi-byte UTF-8 bytes; digit runs straddling the
  4096-byte buffer at 4095/4096/4097/8191/8192/8193; 200 KB inputs; regular-file,
  pipe, `/dev/null`, closed and directory stdin; closed stdout; extra `argv`.
* Exhaustive sweep of all length-1 and length-2 strings over
  `0123456789+- \t\na x NUL`, plus all length-3 strings over `0-+ 1\n9a`.
* 700 seeded pseudo-random inputs per run, plus an out-of-band fuzz of 1440
  additional cases — 0 mismatches.
* The suite was mutation-tested to confirm it is not vacuous. Nine mutants were
  introduced into `src/main.rs` one at a time and each was caught: removing the
  `SIGPIPE` reset, removing the offset restore, changing the buffer size to 8192,
  replacing truncation with saturation, making `bad()` pass `None`, restricting
  the whitespace set to `fgets`-like behavior, and removing either `ungetc` call.
  `src/main.rs` was byte-identical to its pre-mutation state afterwards
  (`diff` clean) and the full suite passed again.
* Nothing in `c_src/` was modified. The only writes under `c_src/` are CMake's
  own outputs in `c_src/build/`, produced by the documented build command;
  `c_src/src/main.c` and `c_src/CMakeLists.txt` are untouched.
