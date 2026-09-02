# Differential verification: `c_src/src/main.c` vs. `translation/`

The C program is the ground truth. This file records what was compared, every
mismatch that was found, and its cause. Fixes were made in the Rust program
only; `c_src/` was not touched.

## How the two programs are run

Both are executables driven exactly as a shell would drive them: no arguments,
input on stdin, results read back from stdout/stderr plus the wait status.

| | command |
|---|---|
| C | `c_src/build/driver` (from `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`) |
| Rust | `translation/target/release/driver` (from `cd translation && cargo build --release`) |

The suite lives in `translation/tests/differential.rs`. It spawns both binaries
as subprocesses and asserts stdout, stderr and exit status all match. The Rust
code is never loaded as a library.

## Mismatches found

### 1. Exit status on a closed stdout: C dies from SIGPIPE, Rust exited 0

**Status: fixed.**

Reproduction — a 100×256-byte reverse (~100 KiB of stdout) piped to a reader
that takes one byte and closes:

```
$ c_src/build/driver   < big.txt > >(head -c 10 >/dev/null); echo $?
141                       # 128 + SIGPIPE(13)
$ translation/.../driver < big.txt > >(head -c 10 >/dev/null); echo $?
0                         # before the fix
```

**Cause.** The Rust standard library installs `SIG_IGN` for `SIGPIPE` before
`main` runs. The C program keeps the platform default, so a `write` to a pipe
with no reader kills it with signal 13. In Rust the same `write` returned
`EPIPE`, the error was discarded, and the process exited normally. The output
bytes and stderr agreed; only the wait status diverged.

This is reachable for real: any output-producing operation (`OP_REVERSE`,
`OP_ROTATE`, `OP_CHECKSUM`, `OP_SPLIT`, `OP_COPY`, `OP_MERGE`,
`OP_INTERLEAVE`) writes enough to fill a pipe buffer at maximum scale.

**Fix.** `translation/src/main.rs` now restores the default disposition as the
first statement of `main`, via a direct `extern "C" { fn signal(...) }`
declaration so no new dependency is introduced:

```rust
fn main() {
    sigpipe::restore_default();   // signal(SIGPIPE, SIG_DFL)
    ...
}
```

Covered by `closed_stdout_kills_both_the_same_way`. That test was confirmed
non-vacuous: commenting out the call makes it fail with
`C signal: 13 (SIGPIPE) vs Rust exit status: 0`.

## Behaviors that look like bugs and were deliberately preserved

Each of these was checked against the C rather than "corrected". They were
already right in the translation; they are listed so the next reader can
re-check them instead of assuming.

- **`scanf("%d")` spans newlines.** The input has no line structure. `1 1 3 1
  2 3` and one token per line produce identical output. All six C `isspace`
  bytes are separators, including `\v` (0x0b) and `\f` (0x0c).
  (`scanf_crosses_newlines_and_accepts_all_whitespace`)
- **Integer overflow truncates, it does not saturate.** glibc converts the
  digits to a `long` (saturating at `LONG_MIN`/`LONG_MAX`) and then stores the
  low 32 bits into the `int`. So `2147483648` becomes `-2147483648`,
  `4294967296` becomes `0` (i.e. `OP_COPY`), and `4294967298` is an accepted
  buffer count of `2`. A saturating implementation is observably wrong here.
  (`scanf_integer_overflow_matches_glibc`)
- **A negative split position becomes a huge `size_t`.** `buffer_split` takes
  `size_t split_pos`, so the `int` read from stdin is sign-extended *before*
  the `split_pos > src->length` check. `-1` is therefore reported as
  `Error: Split position 18446744073709551615 exceeds length 3`, not as a
  negative number. (`op_split`)
- **Byte values wrap to `uint8_t`.** `buf->data[i] = (uint8_t)byte` truncates;
  `256`→0, `-1`→255, `300`→44. No range check and no clamping.
  (`byte_values_are_truncated_to_u8`)
- **`%d` stops at the first non-digit and leaves it.** `0x1f` parses as `0`,
  and the leftover `x1f` makes the *next* conversion fail. `5-3` parses as `5`
  followed by `-3`. (`scanf_number_syntax_quirks`)
- **Order of validation.** All buffers are read before the operation switch is
  reached, so an unknown operation still reports a malformed buffer first:
  `99 1 / -1` gives `Error: Invalid buffer length -1`, not
  `Error: Unknown operation 99`. (`unknown_operations`)
- **`buffer_rotate` short-circuits on length 0** before the `% (int)buf->length`,
  which is what keeps it from dividing by zero. `OP_ROTATE` on an empty buffer
  succeeds and prints `0`. (`op_rotate`)
- **`buffer_interleave` computes `max_len` before the overflow check**, so the
  check still fires; the message has no value in it
  (`Error: Interleaved length exceeds maximum`) unlike the merge message which
  does (`Error: Merged length 257 exceeds maximum`). (`op_interleave`)
- **`OP_SPLIT`'s `buffer_count >= 1` guard is dead.** `buffer_count` is already
  validated `> 0`, so the `else` branch is unreachable and `result` stays 0.
- **Uninitialized `buffer_t` locals.** `temp`, `merged`, `part1`, `part2` are
  uninitialized stack objects in the C. Every operation writes `length` and the
  first `length` bytes before anything reads them, and `write_buffer` only
  prints that prefix, so the Rust zero-initialized `Buffer::new()` is
  observationally identical.
- **`process_buffer_array`, `buffer_conditional_copy` and
  `buffer_copy_strided` are never called by `main`.** They are dead code in the
  C and are kept as dead code in the Rust, so no input can reach them.
- **`validate_buffer`'s checksum warning is unreachable from `main`.**
  `read_buffer` always recomputes the checksum it stores, and the only caller
  (`buffer_copy`) validates a buffer that came straight from `read_buffer`.
  Likewise the `length > 256` branch cannot fire, because `read_buffer`
  rejects any length above 256 first.

## Enumerated input classes, all verified identical

Header: empty input; whitespace only; non-numeric operation; EOF or garbage at
the buffer count; count `0`, `-1`, `101` (rejected); count `1` and `100`
(accepted boundaries); count that overflows into a valid value.

`read_buffer`: length `-1`, `257`, `100000` (rejected); `0`, `1`, `256`
(accepted boundaries); EOF and garbage at each byte index; byte values at and
past the `uint8_t` boundary.

Per operation: `OP_COPY` with 1 buffer (error) and ≥2; `OP_REVERSE` on empty,
single-byte, even, odd and 256-byte buffers; `OP_MERGE` with 1 buffer (error),
sums of 255/256/257/512; `OP_SPLIT` at 0, mid, `length`, `length+1`, negative,
`INT_MAX`, `INT_MIN`, plus a missing/garbage position; `OP_INTERLEAVE` with 1
buffer (error), unequal lengths, empty operands, sums of 256/257/512;
`OP_ROTATE` by 0, 1, `length`, `> length`, negative, `INT_MAX`, `INT_MIN`, on
an empty buffer, plus a missing/garbage amount; `OP_CHECKSUM` including the
`uint32_t` wraparound of `(sum << 3) ^ byte`; unknown operations `7`, `8`,
`-1`, `-3`, `99`, `INT_MAX`.

Process level: `argv` ignored by both; stdin from `/dev/null`; stdin delivered
in 7-byte chunks; tokens zero-padded so conversions straddle an internal buffer
refill; a single 9000-byte token; invalid UTF-8 and NUL bytes in the stream;
maximum scale (100 buffers × 256 bytes, ~100 KiB of stdout).

Randomized: 600 structured cases over the operation/count/length/byte boundary
values and 600 free-form token-soup cases, all requiring identical stdout,
stderr and status.

## Test-suite strength check

To confirm the suite is not passing by accident, 19 deliberate mutations were
injected into `translation/src/main.rs` one at a time and the relevant test was
re-run. 18 were caught, including: wrong checksum shift, wrong merge and
interleave bounds, off-by-one in the length and count guards, dropped sign
extension in `OP_SPLIT`, saturating instead of truncating byte and `scanf`
conversions, a missing trailing newline, a lower-cased error message, a wrong
byte index in an error message, `\v` dropped from the whitespace set, and the
removed `SIGPIPE` restore.

The single surviving mutation replaced `positions % len` followed by
`if positions < 0 { positions += len }` with `positions.rem_euclid(len)`. That
survival is correct: for a positive divisor the two are equal by definition, so
the mutation changes no observable behavior.

## Result

`cargo test` in `translation/`: 29 tests, 29 passed, 0 failed, 0 ignored. No
test is disabled, skipped or `#[ignore]`d. No file under `c_src/` was modified
(only the `c_src/build/` output directory was created, as the build instructions
require).
