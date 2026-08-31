# Verification log: `c_src/src/main.c` → `translation/src/main.rs`

The C program is the ground truth. This file records every behavioural mismatch
found between it and the Rust translation, what caused it, and how it was fixed.
Only `translation/` was changed; nothing in `c_src/` was modified.

## The reference program

```c
void driver(int x) {
    register int y = 2*x;
    y += 300;
    printf("%d\n", y);
}

int main() {
    int x = 0;
    scanf("%d", &x);
    driver(x);
    return 0;
}
```

There is no `if` statement in the source, so every branch lives inside
`scanf("%d", &x)` and inside the signed arithmetic. Exit status is always `0`
and stderr is always empty.

## How it was verified

| Command | Purpose |
| --- | --- |
| `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` | builds `c_src/build/driver` |
| `cd translation && cargo build --release` | builds `translation/target/release/driver` |
| `cd translation && cargo test` | runs the differential suite |

`translation/tests/differential.rs` spawns both binaries as subprocesses, pipes
the same bytes to each, and compares stdout, stderr and exit status. The Rust
crate is never loaded as a library.

Coverage beyond the hand-written cases: all 256 possible leading bytes (alone
and followed by digits), 648 lead/body/tail combinations, 1500 pseudo-random
byte strings, random digit strings of every length from 1 to 25, and a
one-integer-at-a-time sweep across both arithmetic overflow thresholds. During
development an external harness cross-checked a further ~6200 inputs, including
sweeps around 2^31, 2^32, 2^63 and 2^64.

The suite was mutation-tested: three separate bugs were injected into
`src/main.rs` (`wrapping_add` → `saturating_add`, disabled `+` sign handling,
and a reinstated read-to-EOF) and each produced a test failure. The source was
restored bit-identically after every injection.

---

## Mismatch 1 — Rust hung on a stdin that never reaches EOF

**Status: found and fixed.** This was the only true mismatch found.

**Symptom.** With an endless input stream the two programs disagreed about
whether they terminate at all:

```
$ yes 1 | ./c_src/build/driver
302                       # exits immediately, status 0

$ yes 1 | ./translation/target/release/driver
                          # never prints, never exits
```

The same divergence appeared whenever the writer kept the pipe open, even for a
single short line:

```
$ sh -c 'printf "5\n"; sleep 30' | ./c_src/build/driver
310                       # exits at once
$ sh -c 'printf "5\n"; sleep 30' | .../translation/target/release/driver
                          # blocked for the full 30s
```

**Cause.** The Rust `main` drained stdin before scanning:

```rust
let mut buf = Vec::new();
if std::io::stdin().read_to_end(&mut buf).is_ok() {
    if let Some(v) = scanf_i32(&buf) { x = v; }
}
```

`read_to_end` cannot return until the writer closes the pipe. C's `scanf` has no
such requirement: a `%d` conversion finishes the moment it sees the character
that terminates the digit run, pushes that character back with `ungetc`, and
returns. It never asks for a byte it does not need, so it is indifferent to
whether stdin is ever closed.

Note that this was invisible to every fixed-input test. Piping a finite buffer
and closing it — which is what a normal test harness does — makes both versions
agree, because EOF arrives immediately. Only an open-ended stream exposes it.

**Fix.** `scanf_i32` now takes a `BufRead` and pulls bytes lazily through a
`peek`/`consume` pair. `peek` calls `fill_buf`, which is at most one `read`
syscall and returns a partially filled buffer as soon as data is available, so
the scan advances on whatever has arrived rather than waiting for the stream to
end. The character that terminates the number is peeked but never consumed,
mirroring `scanf`'s `ungetc`.

**Confirmation.** The fixed version now matches C on all three sub-cases,
including the ones where C itself blocks — which it must, since it cannot rule
out further input:

| stdin | C | Rust (after fix) |
| --- | --- | --- |
| endless `"1\n"` | prints `302`, exits 0 | identical |
| endless `"42 "` | prints `384`, exits 0 | identical |
| endless `"y\n"` (matching failure) | prints `300`, exits 0 | identical |
| endless `" "` (never a digit) | blocks forever | blocks forever |
| endless `"1234567890"` (never a delimiter) | blocks forever | blocks forever |
| `"5"` with no delimiter, pipe held open | blocks forever | blocks forever |

Regression tests: `endless_stdin_that_terminates_the_number`,
`endless_stdin_that_fails_to_match`,
`endless_stdin_that_legitimately_blocks`.

---

## Behaviours that were checked and already correct

These are recorded because each is a place where a translation would plausibly
go wrong, and each is pinned by a test.

**`scanf` reads across newlines.** `%d` skips leading whitespace of any amount,
newlines included, so `"\n\n\n42\n"` converts `42`. A line-oriented reader such
as `fgets` would have stopped at the first newline and produced `300`.
(`scanf_reads_across_newlines_unlike_fgets`)

**Failed conversion leaves `x` at its initialiser.** `int x = 0;` is written
before `scanf`, and `scanf` does not store anything on input failure (EOF) or
matching failure. Both therefore print `300`, not an error and not a nonzero
exit. Inputs in this class: empty stdin, whitespace only, `"abc"`, a lone `"-"`
or `"+"`, `"--1"`, `"."`, a leading NUL or `0xff` byte, and a closed or
unreadable stdin. Note that `"0x10"` lands here only partly — `%d` converts the
`0` and leaves `"x10"` unread, giving `300` by a different route.
(`matching_failure_leaves_x_at_zero`, `whitespace_only_input`, `empty_input`)

**No error path exists.** `main` returns `0` unconditionally and nothing writes
to stderr, so every input — valid, malformed or absent — yields exit status 0
and empty stderr. The tests assert all three streams on every input rather than
just stdout, since a translation that reported malformed input as an error
would otherwise pass.

**Leading zeros are decimal.** `%d` is base 10 regardless of a `0` prefix, so
`"010"` is ten and `"000000000000000000000000000123"` is 123.
(`leading_zeros_are_decimal_not_octal`)

**Conversion stops at the first non-digit and only one number is read.** `"3 4"`
prints `306`, using only the `3`. (`conversion_stops_at_first_non_digit`)

**glibc saturates in `long`, then truncates to `int`.** The conversion is
performed into a `long`; on overflow glibc stores `LONG_MAX`/`LONG_MIN` and sets
`ERANGE`, and `%d` truncates that `long` to `int`. Two truncations therefore
stack up, and the results are not intuitive:

| stdin | value stored in `x` | printed |
| --- | --- | --- |
| `2147483648` (`INT_MAX + 1`) | `0` | `300` |
| `4294967297` (`UINT_MAX + 2`) | `1` | `302` |
| `9223372036854775808` (`LONG_MAX + 1`) | `-1` (from `LONG_MAX`) | `298` |
| `-9223372036854775809` | `0` (from `LONG_MIN`) | `300` |
| `999999999999999999999999999999` | `-1` | `298` |

The Rust accumulates into an `i64`, clamps to `i64::MIN`/`i64::MAX` on the first
overflow, keeps consuming the remaining digits, and then casts with `as i32`.
Digit runs of 64, 1024, 4095, 4096, 4097 and 100 000 characters were checked, so
the saturating path is exercised well past any buffer boundary.
(`int_boundaries`, `long_boundaries_saturate_then_truncate`,
`very_long_digit_runs`)

**Signed overflow in `driver` wraps.** `2*x` and `y += 300` are `int`
operations that gcc and clang compile to two's complement wraparound, so
`x = 1073741674` prints `-2147483648` and `x = INT_MAX` prints `298`. The Rust
uses `wrapping_mul`/`wrapping_add`; plain `*` and `+` would panic in a debug
build and `saturating_add` gives the wrong answer, which the mutation test
confirmed. `cargo test` runs the debug profile with overflow checks enabled and
passes, so no unintended overflow remains. The boundary is walked one integer at
a time in both directions. (`arithmetic_overflow_wraps`,
`overflow_threshold_sweep`)

**`register` is only a hint.** It has no observable effect and is translated as
an ordinary local.

**`argv` is never read.** Extra command-line arguments change nothing, so the
tests pass argv through to both programs and compare.
(`command_line_arguments_are_ignored`)

**Output format is exactly `"%d\n"`.** One decimal line, no padding or prefix, a
single trailing newline, and nothing on stderr. The C program's output shape is
asserted literally as `b"300\n"` so that a change in the reference is noticed
rather than silently mirrored. (`output_is_exactly_one_decimal_line`)

**Write errors are ignored by both.** C does not check the return value of
`printf`; the Rust discards the `writeln!` result to match. With stdout
redirected to `/dev/full` both still exit `0`.

## Known non-divergence: bytes left unread on stdin

Neither implementation consumes exactly the same number of bytes from the stdin
file descriptor. glibc fills a stdio buffer of `st_blksize` (typically 4096
bytes) and pushes back only the single terminating character, so it over-reads
too; the Rust `BufRead` uses an 8 KiB buffer. The leftover file offset would
only be observable to a process that shares the descriptor and reads after the
child exits. It affects neither stdout, stderr nor exit status, so it is out of
scope for this comparison — but reading byte-at-a-time to "fix" it would itself
be a divergence from glibc, so it was deliberately left alone.
