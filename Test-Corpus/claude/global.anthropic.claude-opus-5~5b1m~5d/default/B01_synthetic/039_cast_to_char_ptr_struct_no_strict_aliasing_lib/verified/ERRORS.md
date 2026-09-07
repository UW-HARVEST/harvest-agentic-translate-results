# ERRORS.md — Phase C error-surface table

Mechanically derived from the whole C source (`c_src/src/driver.c`,
`c_src/include/driver.h`). Greps performed:

```
grep -nE 'return|assert|NULL|errno|if|\?|<|>|exit|abort|ERROR|-1' c_src/src/driver.c
```

Findings: the C library contains **no error-return statements, no error enums,
no `assert`, no null checks, no range checks, and no min/max constants**.
`driver` returns `void`, and `print_hex` returns `void`. There is no allocation,
no I/O that can fail, and no input validation whatsoever.

Consequently the "rejection" rows below are the *implicit* boundaries of the one
public entry point: `void driver(int x)` accepts **every** `int` bit pattern and
must not reject any of them. Each row is still tested differentially: both
libraries are called with the exact input and their observable behaviour
(complete stdout bytes + normal, non-trapping return) must be identical.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `driver` | `x == 0` (zero-length-analogue / all-zero input) | no error; prints `00000000030000000000000000000040\n`; returns normally | `err_e1_zero` | [x] |
| E2 | `driver` | `x == -1` (all-ones / classic sentinel value) | no error; prints the two's-complement bytes `ffffffff…`; returns normally | `err_e2_minus_one` | [x] |
| E3 | `driver` | `x == INT_MAX` (`2147483647`, top of valid `int` range) | no error; prints `ffffff7f…`; returns normally | `err_e3_int_max` | [x] |
| E4 | `driver` | `x == INT_MIN` (`-2147483648`, bottom of valid `int` range) | no error; prints `00000080…`; returns normally | `err_e4_int_min` | [x] |
| E5 | `driver` | `x == INT_MAX + 1` passed across FFI, i.e. the `unsigned` bit pattern `0x80000000` reinterpreted as `int` — one step past the documented positive range | no error; behaves exactly as `INT_MIN` (same bytes); returns normally | `err_e5_one_past_int_max` | [x] |
| E6 | `driver` | `x == 0xFFFFFFFF` (unsigned `UINT_MAX` bit pattern, one step past `INT_MAX` wrap) | no error; behaves exactly as `-1`; returns normally | `err_e6_uint_max_pattern` | [x] |
| E7 | `driver` | out-of-range "enum-like" ints: there is no enum parameter, so the analogue is an arbitrary int with no special meaning (`3`, `0x2a`, `0x7ffffffe`, `-1000000`, …) — the C has **no** `switch`/`if` on `x`, so every value must take the identical single path | no error; prints `x` little-endian followed by the fixed tail; returns normally | `err_e7_arbitrary_no_special_case` | [x] |
| E8 | `driver` | called repeatedly / re-entered many times in one process (no state to corrupt, but confirms no hidden global/static error state) | no error; every call prints an independent, value-determined line | `err_e8_repeated_calls_no_state` | [x] |
| E9 | `driver` (via `print_hex`) | the internal `len` argument: `print_hex` loops `for (i = 0; i < len; i++)` with no bound/null check. `driver` always passes `sizeof(raw) == 16` and a non-null pointer, so a zero/negative `len` or a NULL `p` is **unreachable** from the public ABI | not reachable through the public API; asserted by requiring the output to always be exactly 16 hex byte pairs + `\n` for every input | `err_e9_len_always_16` | [x] |

## Generic FFI boundary notes

* `driver` takes no pointers, so there is no null-pointer row to test other than
  the unreachable internal `print_hex` case (E9).
* `driver` has no length/size parameter, so "zero and oversized lengths" reduce
  to the internal fixed `sizeof(house_t) == 16` (E9).
* `driver` has no enum parameter; the closest real class of bug — an integer
  argument with no corresponding valid variant — is covered by E5/E6/E7, which
  pass bit patterns outside any "expected" value set across the FFI boundary.
* Neither library prints to `stderr` or sets `errno` in any path; both are
  checked for an empty `stderr` in E1–E8 by virtue of only `stdout` being
  redirected and compared, and by both calls returning normally (a trap or
  abort would fail the test process).
