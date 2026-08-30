# Differential verification of the C → Rust translation

Reference: `c_src/src/main.c` (built with CMake to `c_src/build/driver`).
Under test: `translation/src/main.rs` (built to `translation/target/release/driver`).

The C source is written with digraphs and `<iso646.h>` spellings, which decode to:

```c
#include <stdio.h>

void driver(int x, int y) {
    int result = x | ~y;
    printf("%d", result);
    puts("");
}

int main() {
    int x = 0, y = 0;
    scanf("%d", &x);
    scanf("%d", &y);
    driver(x, y);
    return 0;
}
```

There are no `if`s, no early `return`s and no length or null checks in the C.
Every observable behaviour difference therefore comes from one of three places:

1. the outcome of each `scanf("%d")` — success, matching failure, or input
   failure (EOF) — whose return value the C **discards**, leaving the variable
   at its initial `0`;
2. glibc's integer conversion for `%d` (saturate at `long`, then truncate into
   `int`);
3. the fact that `printf`/`puts` return values are also discarded, so write
   failures are invisible to the program.

`translation/tests/differential.rs` enumerates all three, running both binaries
as subprocesses and comparing stdout, stderr and exit status (including
termination by signal) for each input.

## Mismatches found and fixed

### 1. Write error on a full device aborted the Rust program

* Input: `5 7` with stdout redirected to `/dev/full`.
* C: exit code `0`, empty stderr. `exit()` flushes `stdout`, the flush fails,
  and because the program never checks `printf`/`puts`/`fflush` the failure is
  discarded.
* Rust (before): exit `134` (`SIGABRT`, because the release profile sets
  `panic = "abort"`) plus a panic message on stderr:
  `called \`Result::unwrap()\` on an \`Err\` value: Os { code: 28, kind: StorageFull, ... }`.
* Cause: the translation used `write!(...).unwrap()` and `flush().unwrap()`,
  promoting an ignored C error into a fatal Rust panic. Both stderr and the exit
  status diverged.
* Fix: discard the results (`let _ = write!(...)`, `let _ = writeln!(...)`,
  `let _ = out.flush()`), matching C's unchecked I/O.
* Covered by `write_error_on_full_device_is_ignored`.

### 2. Broken stdout pipe terminated with the wrong signal

* Input: `5 7` with stdout being the write end of a pipe whose read end is
  already closed.
* C: killed by `SIGPIPE` (signal 13, shell status `141`), empty stderr.
* Rust (before): exit `134` (`SIGABRT`) plus the `BrokenPipe` panic message on
  stderr.
* Cause: the Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main`, so the
  write returned `EPIPE` instead of killing the process; the `unwrap` then
  aborted. Two independent divergences: the disposition of `SIGPIPE` and the
  unchecked-write issue above.
* Fix: call `signal(SIGPIPE, SIG_DFL)` at the top of `main` (declared through a
  bare `extern "C"` block, no new dependency), in addition to the fix in 1.
* Covered by `broken_pipe_terminates_the_same_way`.

Both were found by driving the binaries the way a shell does; neither is
reachable through stdin bytes alone.

## Behaviours confirmed identical (no change needed)

These were verified rather than assumed, and each has a test. Several are
glibc quirks that a naive translation gets wrong, so they are listed explicitly.

* **`scanf` reads across newlines.** `%d` skips arbitrary leading whitespace
  (space, tab, `\n`, `\r`, `\v`, `\f`), so `"11\n22"` yields `x=11, y=22`. A
  line-oriented (`fgets`-style) reader would leave `y` at `0`.
  (`scanf_reads_across_newlines_unlike_fgets`)
* **Discarded return values.** Empty input, whitespace-only input, and a single
  integer all leave the unread variable at `0`; the program still prints and
  exits `0`. Empty input prints `-1` (`0 | ~0`).
  (`no_input_and_whitespace_only`, `one_integer_only_second_scanf_hits_eof`)
* **Matching failure pushes input back for the *next* `scanf`.** After a failed
  conversion, the offending character is ungotten, so the second `scanf` re-reads
  it. glibc also restores a consumed sign character, which makes these
  distinguishable:
  * `"--5 1"` → `x=0` (first `scanf` fails on the second `-`), then the second
    `scanf` re-reads `-5`, so `y=-5` and the output is `4`, **not** the `-2` you
    would get from `x=0, y=1`.
  * `"+-5 1"` behaves the same way (`4`).
  * `"++5 1"` and `"-+5 1"` both give `y=5` → `-6`; the leading sign is dropped
    and the pushed-back `+` is re-parsed.
  * `"- 5"` and `"+ 5"` → `x=0, y=5` → `-6`.
  * `"-a 5"` → both `scanf`s fail → `-1`.
  The translation's single-byte `ungetc`-style pushback reproduces all of these.
  (`sign_without_digits`)
* **Overflow is saturate-then-truncate, not wrap.** glibc converts `%d` via
  `strtol`, clamps to `LONG_MAX`/`LONG_MIN`, then assigns to `int`, keeping the
  low 32 bits:
  * `"999999999999999999999999999999 1"` → `x = LONG_MAX & 0xFFFFFFFF = -1`, and
    `-1 | ~1 = -1`.
  * `"-999999999999999999999999999999 1"` → `x = LONG_MIN & 0xFFFFFFFF = 0`, and
    `0 | ~1 = -2`. A wrapping or saturating-to-`i32` translation would print
    something else here.
  * `"4294967296 4294967296"` → both truncate to `0`.
  Verified up to 1000-digit runs and 4096 leading zeros.
  (`long_boundaries_and_overflow_truncation`, `very_long_digit_runs`,
  `int_boundaries`)
* **Output format.** `printf("%d", result)` followed by `puts("")` produces the
  decimal digits and exactly one trailing `\n`, with no space or padding.
  (`both_binaries_run`)
* **`x | ~y` on 32-bit `int`.** Swept over sign and bit-pattern boundaries
  including `INT_MIN`/`INT_MAX` and alternating bit masks; the expression cannot
  overflow, so no UB is involved.
  (`every_bit_pattern_class_of_the_or_not_expression`,
  `deterministic_integer_pair_sweep` — 21 × 21 pairs)
* **Non-numeric and hostile bytes.** Letters, `0x10` (reads `0`, then fails on
  `x`), `12abc34`, `.5`, `1e5`, `1_2`, commas, embedded NUL bytes, `0xFF`, and
  full-width Unicode digits all follow the same matching-failure path.
  (`matching_failure_on_non_numeric_input`)
* **Trailing input is ignored.** Anything after the second integer is never
  read; the program exits before consuming it, and both binaries tolerate the
  resulting `EPIPE` on the writer side identically.
  (`trailing_input_after_two_integers_is_ignored`)
* **Argv is ignored; `/dev/null` and a closed stdin behave as EOF.**
  (`command_line_arguments_are_ignored`, `stdin_from_dev_null`)

## Status

* `c_src/` is unmodified.
* `cd translation && cargo build --release` — clean, no warnings.
* `cd translation && cargo test` — 18 tests, all passing, none `#[ignore]`d,
  skipped or disabled. (`write_error_on_full_device_is_ignored` returns early
  only if `/dev/full` does not exist, which is not the case on this Linux host.)
* An additional 6000-case randomized differential sweep (outside the test
  suite, alphabet of digits, whitespace, signs and separator/garbage bytes)
  found no further differences; a 400-case deterministic version of the same
  sweep is kept in the suite as `randomized_byte_sweep`.

Commands used:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                # -> translation/target/release/driver
cd translation && cargo test
```
