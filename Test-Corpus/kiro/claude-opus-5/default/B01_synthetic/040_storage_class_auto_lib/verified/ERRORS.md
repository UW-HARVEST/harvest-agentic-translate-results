# ERRORS.md — Phase A error-surface table

Derived mechanically from the C source, not from documentation.

## Mechanical grep evidence

The whole library, comments stripped:

```c
#include "driver.h"
#include <stdio.h>
void driver(int x) {
    auto int y = 2*x;
    y += 300;
    printf("%d\n", y);
}
```

Greps over `c_src/src/driver.c` + `c_src/include/driver.h` for every rejection
construct:

| construct searched for | matches |
|------------------------|---------|
| `return` (any), `return -1`, `return NULL` | 0 (function is `void`, has no `return`) |
| `RETURN_ERROR` / any error macro | 0 |
| `assert` / `static_assert` / `abort` / `exit(` | 0 |
| `errno`, error enums, status codes | 0 |
| `if (` / `switch` / `?:` / any comparison (`<`, `>`, `==`, `!=`) | 0 |
| `#if` / `#ifdef` in the source (only the `DRIVER_H_` include guard in the header) | 0 functional |
| pointer parameters (→ null checks) | 0 — the only parameter is `int x` |
| min/max constants, range checks, length/size checks | 0 |

**The C library has no explicit rejection path at all**: no error return (the
function returns `void`), no output parameter, no pointer argument that could be
NULL, no length that could be zero or oversized, and no enum parameter. Every
`int` in `[INT_MIN, INT_MAX]` is an accepted input and always produces exactly
one line of output on `stdout`.

Consequently the table below has no `RETURN_ERROR`-style rows. Per Phase C's
requirement to also cover "the generic boundaries every C API has even if not in
the table", the rows are the *implicit* rejection/edge conditions that actually
exist in this code: the two signed-overflow points and the extremes of the
parameter domain. "Expected C result" is the observable behaviour of the
reference build (`gcc`, no optimisation flags, i.e. two's-complement wraparound),
and is asserted differentially against the Rust `.so` rather than hard-coded.

## Error / boundary surface

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `driver` | `x = INT_MAX` (2147483647) — signed overflow in `2*x` (UB in C; wraps in the reference build) | prints `298\n`, returns normally, no abort/trap |
| 2 | `driver` | `x = INT_MIN` (-2147483648) — signed overflow in `2*x` | prints `300\n`, returns normally |
| 3 | `driver` | `x = INT_MAX/2 + 1 = 1073741824` — smallest positive `x` where `2*x` overflows | prints `-2147483348\n` |
| 4 | `driver` | `x = INT_MAX/2 = 1073741823` — largest `x` where `2*x` does **not** overflow, but `y += 300` does | prints `-2147483350\n` |
| 5 | `driver` | `x = 1073741673` — largest `x` for which neither `2*x` nor `y += 300` overflows | prints `2147483646\n` |
| 6 | `driver` | `x = 1073741674` — one step past row 5: first `x` whose `y += 300` overflows | prints `-2147483648\n` |
| 7 | `driver` | `x = INT_MIN/2 = -1073741824` — most negative `x` where `2*x` does not overflow | prints `-2147483348\n` |
| 8 | `driver` | `x = INT_MIN/2 - 1 = -1073741825` — one step past row 7, `2*x` overflows negatively then `+300` overflows | prints `-2147483350\n` |
| 9 | `driver` | `x = -150` — result is exactly `0`; checks `printf("%d")` zero formatting (no sign, no leading blank) | prints `0\n` |
| 10 | `driver` | `x = -151` — result is `-2`, smallest-magnitude negative output; checks minus-sign formatting | prints `-2\n` |
| 11 | `driver` | out-of-range "enum"/garbage bit pattern passed across FFI: `x` = raw `0xFFFFFFFF` reinterpreted as `int` (`-1`) and `x = 0x80000000` (`INT_MIN`) — C `int` accepts any 32-bit pattern, there is no valid-variant check to fail | prints `298\n` and `300\n` respectively; no rejection (the C performs no validation, so the Rust must not either) |
| 12 | `driver` | truncation boundary: caller passes a 64-bit value whose low 32 bits are `1` (`0x1_0000_0001`) through the `int` parameter | both sides observe only the low 32 bits → prints `302\n` (identical ABI truncation in C and Rust) |

All 12 rows are covered by `tests/error_paths.rs`, which builds each condition,
calls **both** `.so`s through `libloading`, and asserts the captured stdout bytes
are identical *and* that neither side aborts/traps (i.e. the same
"non-rejection" behaviour, byte-for-byte, not merely "both failed somehow").

## Verification result (Phase C)

Every row above has a passing differential test in `tests/error_paths.rs`, and
the "expected C result" column is asserted as an exact literal in that test *in
addition to* the C-vs-Rust byte comparison, so the table itself is machine-checked:

| row | test |
|-----|------|
| 1 | `err01_int_max` |
| 2 | `err02_int_min` |
| 3 | `err03_first_mul_overflow` |
| 4 | `err04_add_overflow_at_max_half` |
| 5 | `err05_last_non_overflowing` |
| 6 | `err06_one_past_last_non_overflowing` |
| 7 | `err07_min_half` |
| 8 | `err08_one_past_min_half` |
| 9 | `err09_zero_result_formatting` |
| 10 | `err10_minus_sign_formatting` |
| 11 | `err11_out_of_range_bit_patterns` |
| 12 | `err12_wide_argument_truncation` |
| generic boundaries (±3 around every boundary above) | `err_generic_boundary_neighbourhoods` |

`test result: ok. 13 passed; 0 failed` — under the default feature set, under
`--no-default-features`, and in both the `dev` (overflow checks ON) and
`release` profiles.

## Harness sensitivity (negative control)

To prove these tests are not vacuous, `src/lib.rs` was temporarily mutated
(`+300` → `+301`, and separately `2*x` → `2*x+1`), rebuilt, and re-run: all 13
error-path tests failed with explicit divergence messages. The mutation was then
reverted and the suite returned to green. Two harness defects were found and
fixed this way:

1. libtest's parallel progress output leaked into the captured fd-1 bytes →
   the test targets now use `harness = false` with a strictly sequential runner.
2. `cargo test` does not rebuild the `cdylib`, so tests could pass against a
   stale `libdriver.so` → the harness now hard-fails on a stale artifact
   (`assert_not_stale`).
