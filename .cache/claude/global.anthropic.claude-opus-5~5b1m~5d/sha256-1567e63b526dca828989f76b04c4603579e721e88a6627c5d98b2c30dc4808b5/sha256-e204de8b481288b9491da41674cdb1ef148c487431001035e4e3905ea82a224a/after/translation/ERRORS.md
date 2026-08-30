# Differential verification report

Ground truth: `c_src/src/main.c` (never modified). The Rust binary must produce
byte-identical stdout, byte-identical stderr and the same exit status.

## How to run each program

```
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
./c_src/build/driver < input

# Rust
cd translation && cargo build --release
./translation/target/release/driver < input
```

Differential suite: `cd translation && cargo test` (26 tests in
`tests/differential.rs`; both binaries are spawned as subprocesses, never
loaded as a library). The test harness builds the C binary with cmake if
`c_src/build/driver` is missing; it only creates the build directory and never
touches `c_src` sources.

## What the C program does

`main` zero-initialises `x, y, b, z`, then runs four unchecked `scanf` calls
(`%u`, `%u`, `%d`, `%d`) and calls `driver(x, y, !!b, z)`. `driver` stores the
values into bit-fields `x:2`, `y:3`, `bool b:1` plus a plain `int z`, and prints
`"%u %u %d %d\n"`. Exit status is always 0 and stderr is always empty, so the
only observable variation is the single stdout line.

Input classes that matter:

| class | behaviour |
| --- | --- |
| 0–4 successfully converted items | unconverted variables keep their initial `0` |
| any whitespace layout (space, tab, `\n`, `\r`, `\v`, `\f`) | `scanf` skips it and reads across newlines |
| matching failure (`abc`, `-`, `+`, `.5`, `0x10`, `1,2`) | destination untouched; offending char stays in the stream, so all later conversions fail too |
| `x >= 4` / `y >= 8` | truncated to 2 / 3 bits (`& 3`, `& 7`) |
| `b != 0` | `!!b` normalises to `1`; `0` prints `0` |
| `%u` with a `-` sign | accepted, negated modulo 2^64 then truncated to 32 bits |
| out-of-range magnitudes | glibc saturates (`ULONG_MAX` for `%u`, `LONG_MIN`/`LONG_MAX` for `%d`) before the assignment truncates to 32 bits |

## Mismatches found

### 1. `%u` conflated "digits overflowed" with "value equals `ULONG_MAX`"

* **Found by:** randomised differential fuzzing (8 476 inputs), reproducer
  `2147483648\r\n-18446744073709551615\t`.
* **C output:** `0 1 0 0` — **Rust output before fix:** `0 7 0 0`.
* **Cause:** `Scanner::scan_int_parts` collapsed the overflow signal into the
  magnitude by clamping it to `u64::MAX`, and `scan_u` then keyed off
  `mag == u64::MAX` to decide "out of range". The literal input
  `-18446744073709551615` has a magnitude that *is* exactly `ULONG_MAX` without
  overflowing, so glibc's `strtoul` negates it (`0 - ULONG_MAX == 1`) and stores
  `1`, giving `y = 1 & 7 = 1`. The Rust code instead treated it as out of range,
  returned `ULONG_MAX` and printed `y = 7`.
* **Fix:** `scan_int_parts` now returns `(negative, magnitude, overflowed)` with
  the overflow flag tracked separately from the magnitude; `scan_u` saturates to
  `ULONG_MAX` only when `overflowed` is set, otherwise it negates modulo 2^64.
  `scan_d` was made consistent in the same commit (`overflowed` ⇒ `LONG_MIN` /
  `LONG_MAX`), which removed a duplicated `mag == i64::MAX + 1` branch that had
  the same latent defect for `%d`.
* **Regression tests:** `values_at_and_beyond_64_bit_limits`
  (`1 -18446744073709551615 1 1`, `-18446744073709551615 1 1 1`,
  `1 -18446744073709551616 1 1`), `sweep_token_in_every_position`,
  `absurdly_long_digit_runs_overflow_the_same_way`.

No other mismatch was observed.

## Behaviours deliberately preserved (not "fixed")

* `scanf` return values are ignored, exactly as in the C. A failed conversion is
  silent and leaves the variable at `0`; the program still exits 0.
* A matching failure consumes a leading `+`/`-` but pushes back the offending
  character, so `- 1 2 3` prints `0 1 1 3` (the `-` is eaten, `1` becomes `y`).
* `%u` accepting a negative sign, and the wrap-around it causes, is reproduced
  rather than rejected.
* Leading zeros are decimal, not octal (`010` reads as ten).
* Values are saturated by the conversion and *then* truncated by assignment, so
  e.g. `1 1 1 99999999999999999999` prints `z = -1` (`LONG_MAX as i32`).

## Test-suite strength check (mutation testing)

Each mutation was injected into `src/main.rs`, `cargo test --release` was run,
and the source was restored. Every mutation was caught:

| mutation | failing tests |
| --- | --- |
| `x & 0x3` → `x & 0x7` | 12 |
| `y & 0x7` → `y & 0xF` | 14 |
| `b` printed as `2` instead of `1` | 23 |
| `b != 0` → `b > 0` | 23 |
| `mag.wrapping_neg()` → `mag` | 23 |
| `if overflow` → `if mag == u64::MAX` (the original defect) | 3 |

## Status

* Both programs build with no errors (`cmake --build .`, `cargo build --release`).
* `cargo test` in `translation/`: 26 passed, 0 failed, 0 ignored — no test is
  disabled, skipped or `#[ignore]`d.
* Beyond the suite, 14 476 fuzzed inputs (three seeds) plus every enumerated
  edge case agree on stdout, stderr and exit status.
* `c_src/` is unmodified; only `c_src/build/` (generated) was created.
