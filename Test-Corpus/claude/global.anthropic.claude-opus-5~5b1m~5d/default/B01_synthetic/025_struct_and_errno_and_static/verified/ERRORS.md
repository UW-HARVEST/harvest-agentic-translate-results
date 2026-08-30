# Differential verification: mismatches found and fixed

The Rust translation in `src/main.rs` was compared against the C in
`c_src/src/main.c` by building both and running them as subprocesses over the
same stdin, then diffing stdout, stderr and the termination status.

- C binary: `c_src/build/driver` (`cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`)
- Rust binary: `translation/target/release/driver` (`cd translation && cargo build --release`)
- Test suite: `translation/tests/differential.rs` (`cd translation && cargo test`)

Total inputs compared: 19 test functions covering ~90 curated input classes,
plus an ad-hoc randomized sweep of 1754 inputs (random values across
`[-2^34, 2^34]`, random byte strings drawn from
`0123456789+- \t\n\r abcxX . NUL 0xff 0x80`, and random 90–110 byte lines to
straddle the `char in[100]` boundary).

---

## Mismatch 1 — writing to a closed stdout: exit 0 vs killed by SIGPIPE

**Status: found and fixed.**

**Symptom.** With stdout being a pipe whose read end had already been closed:

| | termination status |
|---|---|
| C (`c_src/build/driver`) | killed by signal 13 (SIGPIPE), i.e. shell status 141 |
| Rust (before fix) | exited 0 |

Reproduction (input `3\n`, `nope\n` and `` all showed it):

```python
r, w = os.pipe()
p = subprocess.Popen([prog], stdin=subprocess.PIPE, stdout=w, stderr=subprocess.DEVNULL)
os.close(w); os.close(r)
p.stdin.write(b"3\n"); p.stdin.close()
print(p.wait())     # C: -13    Rust: 0
```

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` before calling
`main`. The C program never does that, so it keeps the default disposition and
is killed when its exit-time `fflush(stdout)` gets `EPIPE`. In Rust the failing
write merely returned `Err(BrokenPipe)`, which the translation discarded (as C's
`printf` discards errors), so the process ran on and returned 0.

**Fix.** Restore the default disposition at the top of `main`, using `signal`
declared through a bare `extern "C"` block (libc is already linked by std, so no
new crate dependency is needed):

```rust
extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
const SIGPIPE: i32 = 13;
const SIG_DFL: usize = 0;
fn restore_default_sigpipe() { unsafe { signal(SIGPIPE, SIG_DFL); } }
```

**Regression test.** `writing_to_a_closed_stdout_is_killed_by_sigpipe`. Verified
to have teeth: with the `signal()` call stubbed out, the test fails with
`C=Status { code: None, signal: Some(13) } R=Status { code: Some(0), signal: None }`.

Note this required comparing the terminating *signal*, not just
`ExitStatus::code()`. The original assertion only looked at `code()`, which is
`None` both for SIGPIPE and for SIGSEGV, so a signal difference could have hidden
there. The test helper now compares a `Status { code, signal }` pair.

## Mismatch 2 — stdout buffering discipline (surfaced by fixing #1)

**Status: found and fixed.**

**Symptom.** Once SIGPIPE was restored, a *partial* reader (one that reads a few
bytes of stdout and then exits) produced a new, nondeterministic divergence:

| | termination status over 40 runs |
|---|---|
| C | always `0` |
| Rust (line-buffered) | mix of `0` and `-13` (SIGPIPE) |

**Cause.** glibc gives `stdout` a *fully* buffered stream when it is not a
terminal (`BUFSIZ` >= 4096). This program emits at most 8 lines of under 80
bytes — under 640 bytes total — so glibc issues exactly one `write(2)`, during
the flush at exit. The Rust translation used `std::io::stdout()`, which is a
`LineWriter` and therefore issued one `write(2)` per line. With SIGPIPE now
fatal, the later per-line writes could land after the reader had gone away and
kill the Rust process, where C had already pushed everything out in a single
write before the reader exited.

**Fix.** Emulate C's stdio buffering: accumulate output in a `thread_local`
`Vec<u8>` and perform a single write at the end of `main` (the implicit
`fflush` that returning from `main` does). `isatty(1)` is consulted so that the
line-buffered behaviour is still used when stdout is a terminal, matching glibc.

**Regression test.** Covered by `writing_to_a_closed_stdout_is_killed_by_sigpipe`
and `write_error_other_than_epipe_is_ignored`. Verified to have teeth: forcing
`stdout_is_line_buffered()` to return `true` reproduces the `{0, -13}` split
above.

---

## Behaviours deliberately preserved (checked, not bugs to "fix")

These all looked wrong at a glance; each was confirmed against the C binary and
is reproduced faithfully rather than corrected.

- **`"An error occurred"` goes to stdout, not stderr, and the program still
  returns 0.** The C never writes to stderr and never returns non-zero, on any
  input. Both programs produce empty stderr and exit 0 in every non-signal case.
- **`endp != str` accepts trailing garbage.** `42abc`, `1 2`, `3.9`, `0x10`
  (base 10, so this is `0` followed by the ignored `x10`), `2e5` and `5-3` are
  all *valid* inputs producing 42, 1, 3, 0, 2 and 5. Covered by
  `trailing_garbage_is_accepted`.
- **`strtol` skips leading whitespace**, so `"   12"`, `"\t12"` and even
  `"\n42"` succeed. Covered by `leading_whitespace_is_skipped_by_strtol`. The
  whitespace set is the C-locale `isspace` set: space, `\t`, `\n`, `\v`, `\f`,
  `\r`.
- **Two distinct rejection reasons.** `2147483648` is rejected by the
  `tmp <= INT_MAX` range check with `errno == 0`, whereas `9223372036854775808`
  is rejected by the `errno == 0` check (`ERANGE`). Both reach the same error
  message, but the translation must reject *both*, so
  `int_range_boundaries` and `strtol_erange` exercise them separately, including
  `LONG_MAX` / `LONG_MIN` which fit a `long` yet fail the int range test.
- **Signed overflow of `bedrooms += extra_bedrooms`.** This is UB in C; the
  binary built by CMake (no `-O` flags, so `-O0`) wraps. The translation uses
  `wrapping_add`, and `bedroom_addition_overflow` pins the observed behaviour
  for `INT_MAX`, `INT_MIN` and values that only overflow on the *second*
  `run()` call.
- **Global state persists across the two `run()` calls.** `run(x); run(x);`
  does not reset `the_house`, so floors go 2→3→4 and bathrooms 2.5→3.5→4.5
  across the 8 printed lines. Pinned by
  `golden_success_path_for_one_extra_bedroom`.
- **`fgets` does not read across newlines** and reads at most 99 bytes into
  `char in[100]`. A long line is truncated and the remainder is left unread and
  ignored. `fgets_100_byte_buffer_limit` covers 99/100 `1`s, a truncation that
  splits a number in half (`" "*95 + "1234567"` parses as **1234**), and 99
  spaces that push the digits entirely out of reach so the input is *rejected*.
  `extra_input_after_first_line` confirms trailing lines never matter.
- **An embedded NUL truncates the C string** even though `fgets` copied the
  later bytes into the buffer: `"\0" + "42"` is rejected, `"42\0abc"` yields 42.
  Covered by `embedded_nul_bytes`.
- **`char in[100] = ""` means a failed `fgets` leaves an empty string**, so EOF,
  a closed stdin, and a stdin that is a directory all take the error path rather
  than reading uninitialized memory. Covered by `empty_and_whitespace_only_input`
  and `stdin_closed_immediately`.
- **`%.1f` vs `{:.1}` rounding.** Not actually reachable as a difference:
  `bathrooms` only ever takes the values 2.5, 3.5, 4.5 and 5.5 here, all of which
  are exactly representable and need no rounding decision at one decimal place.
  Noted in a comment in `print_the_house` so it is not mistaken for luck.
- **A failed flush that is not `EPIPE` is silently ignored.** With stdout on
  `/dev/full` the exit-time flush fails with `ENOSPC`; C's `exit` does not report
  it and returns 0. `write_error_other_than_epipe_is_ignored` checks both
  programs stay silent on stderr and exit 0.
- **Non-UTF-8 input must not panic.** The translation works on `&[u8]`
  throughout and never calls `from_utf8`, so `\xff\xfe42` and lone continuation
  bytes behave as C does. Covered by `non_utf8_bytes`.

## Final state

`cargo test` in `translation/`: 19 passed, 0 failed, 0 ignored. No test is
disabled, skipped or `#[ignore]`d. Nothing in `c_src/` was modified.
