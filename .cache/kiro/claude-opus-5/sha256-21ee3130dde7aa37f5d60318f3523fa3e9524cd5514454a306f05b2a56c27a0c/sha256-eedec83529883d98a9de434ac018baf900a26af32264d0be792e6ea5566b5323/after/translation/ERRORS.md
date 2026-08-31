# Differential verification log

Ground truth: `c_src/src/main.c`, built with CMake/GCC on Linux (glibc).
Under test: `translation/` (`src/main.rs`).

Commands used to run each program:

```
c_src/build/driver                      # after: cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
translation/target/release/driver        # after: cd translation && cargo build --release
```

Both are driven as subprocesses with the same bytes on stdin; stdout, stderr and
exit status (including death by signal) are compared. See
`tests/differential.rs` and `tests/common/mod.rs`.

---

## Mismatches found

### 1. `SIGPIPE` was ignored, so the Rust program survived a write the C program died on

**Status:** fixed in `src/main.rs` (`reset_sigpipe`).

**Symptom.** With stdout connected to a pipe whose read end has already been
closed:

| | stdout | stderr | exit status |
|---|---|---|---|
| C | empty | empty | killed by signal 13 (`SIGPIPE`) |
| Rust (before) | empty | empty | exit 0 |

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` during runtime
startup, before `main` is entered. The failing `write` therefore returned
`EPIPE` instead of terminating the process, and `print_foo` discards the write
result (`let _ = write!(...)`), so the program ran to a normal `return 0`. The C
program keeps the default disposition and is killed by the signal.

**Fix.** Restore `SIG_DFL` for `SIGPIPE` as the first statement of `main`, by
declaring `signal(2)` with `extern "C"` — the standard library already links the
platform C library, so this needs no new dependency.

**Covered by:** `dying_from_sigpipe_when_stdout_has_no_reader`.

**Note.** This is the only difference found. It is not reachable through stdin
content; it only appears when the consumer of stdout goes away. A harness that
reads the child's stdout to completion will never see it, which is why it
survived the input-driven cases.

---

## Behaviour deliberately preserved, including the parts that look like bugs

None of these required a change — the translation already matched — but each was
confirmed against the C binary rather than assumed, and each is pinned by a test.

- **A failed `scanf` leaves its destination alone.** `x`, `y`, `b` and `z` are
  initialised to `0` and `main` ignores every `scanf` return value, so a failed
  conversion silently yields `0`. Empty input prints `0 0 0 0` and exits 0; there
  is no error message and no non-zero exit anywhere in this program.
- **`scanf` reads across newlines.** Leading whitespace — including `\n`, `\r`,
  `\t`, `\v` and `\f` — is skipped by every numeric directive, so `"1\n2\n3\n4"`
  is read exactly like `"1 2 3 4"`. This is the `scanf`/`fgets` distinction and
  it is what the C does.
- **A matching failure poisons every later conversion.** The byte that stopped
  the scan is pushed back into the stream, so it stops the next directive too.
  `"1 2 x 4"` prints `1 2 0 0`, not `1 2 0 4` — the `4` is never reached. The
  Rust `Scanner` reproduces this with a one-byte pushback.
- **An accepted sign is consumed even when the conversion then fails.** For
  `"--1 2 3 4"` the first `%u` eats the first `-`, fails on the second, and
  pushes only that second `-` back; the next `%u` then reads `-1`. Result:
  `0 7 1 3`.
- **`%u` accepts a sign and wraps.** glibc converts with `strtoul`, so `-1`
  becomes `ULONG_MAX`, truncates to `0xFFFFFFFF`, and the 2-bit field prints `3`.
  `-18446744073709551615` wraps to `1`.
- **Out-of-range values saturate at 64 bits, then truncate to 32.** `%u`
  saturates to `ULONG_MAX`; `%d` saturates to `LONG_MAX`/`LONG_MIN`. So
  `"0 0 0 99999999999999999999999"` prints `-1` (`LONG_MAX` truncated to `int`)
  and the negative counterpart prints `0` (`LONG_MIN` truncated to `int`).
- **In-range-for-64-bit values truncate rather than saturate.** `4294967296` for
  `%u` gives `0`; `2147483648` for `%d` gives `-2147483648`.
- **Bit-field widths are masks, not errors.** `unsigned int x : 2` keeps
  `x & 0x3`, `unsigned int y : 3` keeps `y & 0x7`; `7 15 …` prints `3 7 …`.
- **`!!b` and `bool b : 1`.** Any non-zero `b` prints `1`, including values that
  are non-zero only before truncation. `%d` on the promoted one-bit `bool`
  prints `0` or `1`.
- **No `%i`, so no hex.** `"0x10"` converts as `0` and leaves `x` in the stream,
  which then breaks the remaining three conversions: output `0 0 0 0`.
- **Byte-oriented input.** A NUL byte is an ordinary non-digit, not a
  terminator, and no byte above `0x7F` counts as whitespace in the C locale.
- **Output shape.** Exactly one line, four fields, single spaces, one trailing
  `\n`, nothing on stderr, exit 0.

---

## Verification performed

- 32 integration tests in `tests/differential.rs`, all comparing stdout, stderr
  and exit status. None is `#[ignore]`d, skipped or disabled.
- Roughly 1,000 randomized inputs per run from two fixed-seed generators
  (numeric combinations and raw byte soup), plus an ad-hoc sweep of about 7,000
  inputs during investigation — zero mismatches after the fix.
- The suite was mutation-checked to confirm it is not vacuous. Removing the
  `y & 0x7` mask, removing the `SIGPIPE` reset, and breaking the negative-`%u`
  wrap each produced failures (10, 1 and 6 failing tests respectively); the
  unmodified translation passes all 32.

## Note on `src/scanf.rs`

`src/main.rs` does not declare `mod scanf;`, so `src/scanf.rs` is not part of the
build. It is a second, unused copy of the same conversion logic. It was left as
found; the behaviour under test comes entirely from `src/main.rs`.
