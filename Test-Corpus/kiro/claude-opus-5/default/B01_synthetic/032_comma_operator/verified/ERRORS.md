# ERRORS.md — mismatches found while verifying the C→Rust translation

Ground truth: `c_src/src/main.c`, built with CMake to `c_src/build/driver`.
Program under test: `translation/src/main.rs`, built to
`translation/target/release/driver`.

Both programs are compared by execution only: same stdin bytes, then a
byte-for-byte diff of stdout, a byte-for-byte diff of stderr, and an exit-status
comparison. Tests live in `translation/tests/differential.rs`.

## What the C program does

```c
void driver(int x) {
    for (int i = 0, j = 0; i < x; i++, j += 2) printf("%d %d\n", i, j);
}
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

The observable behavior therefore hinges on exactly three things: how `%d`
parses (or fails to parse) stdin, the `i < x` loop bound, and the fact that
`main` always returns 0.

## Mismatch 1 — SIGPIPE disposition (found in Phase C, fixed)

**Symptom.** When the reader of stdout goes away before the program finishes
writing, the two programs ended differently:

| | exit status |
|---|---|
| C | killed by signal 13 (`SIGPIPE`), shell reports 141 |
| Rust (before fix) | `0` |

Reproduction:

```
$ echo 1000000 | c_src/build/driver            | head -c 20 >/dev/null; echo ${PIPESTATUS[1]}
141
$ echo 1000000 | translation/target/.../driver | head -c 20 >/dev/null; echo ${PIPESTATUS[1]}
0
```

**Cause.** The Rust standard library installs `SIG_IGN` for `SIGPIPE` during
runtime setup, before `main` runs. A C program inherits the default
disposition. With `SIGPIPE` ignored, the failing `write` merely returns `EPIPE`;
that error was discarded (`let _ = writeln!(...)`, `let _ = out.flush()`), so
the Rust program ran to completion and exited 0 where the C program was killed
mid-write.

Note that this is not a formatting or parsing bug — stdout content matched. Only
the exit status diverged, which is exactly the class of defect a stdout-only
test would have missed.

**Fix.** `translation/src/main.rs` now restores the default disposition as the
first statement of `main`, via a direct `signal(SIGPIPE, SIG_DFL)` call (no new
dependency; `signal` is already linked):

```rust
#[cfg(unix)]
fn restore_default_sigpipe() { /* signal(13, SIG_DFL) */ }
```

**Regression coverage.** `dead_stdout_reader_kills_both_programs_alike` closes
the read end of stdout while the child is still blocked in the `%d` conversion,
making the first attempted write deterministically fail for both programs;
`stdout_reader_closes_partway_through_a_long_run` reads 16 bytes of a 5,000,000
line run and then drops the pipe. Both tests were confirmed to fail against the
pre-fix binary (`left: Err(Some(13))`, `right: Ok(0)`) and to pass after it.

## Behaviors that were verified as already correct

These were checked because they are the usual places a translation drifts, not
because they were broken. All matched on the first run.

- **`%d` skips whitespace across newlines.** `scanf` is not `fgets`: input
  `"\n\n  7"` yields 7. Also verified for tab, `\r`, vertical tab and form feed,
  and for 2000 leading newlines and 9000 leading spaces (past any stdio buffer
  boundary).
- **Failed conversion leaves `x` at its initializer.** Empty input, whitespace
  only, `"abc"`, `"+"`, `"-"`, `"-x"`, `"+ 5"`, `"--5"`, a leading NUL and
  leading `0xff`/`0x80` bytes all leave `x == 0`, so nothing is printed and the
  exit status is 0. The return value of `scanf` is ignored by the C, so a
  failure is indistinguishable from reading 0 — the Rust matches by also
  ignoring it.
- **`%d` is decimal only.** `"0x10"` reads 0 (stops at `x`), `"12.9"` reads 12
  (stops at `.`), `"3e2"` reads 3, `"007"` reads 7 — not octal.
- **Trailing garbage is left unread.** `"3abc"`, `"3 4"`, `"2-3"` all read 3, 3
  and 2. Nothing else consumes stdin, so the pushback slot is unobservable, but
  it is implemented so the reader never over-consumes.
- **glibc's overflow rule: accumulate in `long`, saturate at
  `LONG_MIN`/`LONG_MAX`, then narrow to `int`.** This is the subtle one, and the
  narrowed value changes the output:
  - `"2147483648"` (2^31) → `INT_MIN` → no output
  - `"4294967296"` (2^32) → 0 → no output
  - `"4294967297"` → 1 → prints `0 0`
  - `"17179869189"` (2^34+5) → 5 → prints 5 lines
  - `"-4294967292"` → 4 → prints 4 lines
  - `"9223372036854775807"` (`LONG_MAX`), `"9223372036854775808"`,
    `"99999999999999999999"` and a 5000-digit run of nines all saturate to
    `LONG_MAX` → narrow to `-1` → no output
- **`printf("%d %d\n", ...)` formatting.** Single space separator, trailing
  newline on every line including the last, no leading padding. Verified across
  1-, 2-, 3-, 4-, 5- and 6-digit columns with counts up to 100000 (~1.2 MB of
  stdout diffed byte for byte, which also confirms the two buffering strategies
  produce the same final byte stream).
- **stderr is always empty and the exit status is always 0** on every
  non-signalled path.
- **Non-UTF-8 stdin** is handled bytewise, not as text: `[0xff, 0xfe, b'5']`,
  embedded NULs, and bare high bytes all match.

## Deliberately not tested

- **`x == INT_MAX`** is the true maximum the loop handles, but it prints ~2^31
  lines in *both* programs and cannot be compared in finite time. The adjacent
  parse boundaries (`INT_MIN` by literal and `INT_MIN` reached by narrowing
  2^31) are tested instead, and the largest count actually executed is 100000.
  Two candidate cases were dropped for this reason after computing their
  narrowed value: `"6442450943"` and `"-2147483649"` both narrow to `INT_MAX`.
- **Signed overflow of `j`** (`j += 2`) is undefined behavior in C and is first
  reached at `i > 2^30`, i.e. only inside runs that are already untestably long.
  The Rust uses `wrapping_add`, which matches what the C compiler emits here,
  but this is unreachable in any test that terminates.

## Randomized cross-check

Beyond the enumerated cases, 4000 pseudo-random inputs (0–14 bytes drawn from
digits, signs, all six whitespace characters, `x`/`X`/`.`/`e`/`E`, punctuation,
NUL, and high bytes) were run through both binaries with stdout, stderr and exit
status compared. Zero mismatches.

## Final state

- Both programs build clean.
- `cargo test` in `translation/`: 22 tests, all passing, none `#[ignore]`d,
  skipped or disabled.
- Nothing in `c_src/` was modified.
