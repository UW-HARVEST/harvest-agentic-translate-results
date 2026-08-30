# ERRORS.md — Phase C error/rejection surface table

Mechanically derived from `c_src/src/driver.c` and `c_src/include/driver.h` by
grepping for every rejection construct:

```
grep -nE 'return|assert|NULL|errno|exit\(|abort|RETURN_ERROR|-1' c_src/src/driver.c
```

**Findings (verbatim):** the only matches are `#include <stdio.h>` /
`#include <stdlib.h>`. The C source contains:

* **no** `return` statements (the function is `void` and falls off the end),
* **no** `assert`,
* **no** `NULL` / pointer checks (the API takes no pointers),
* **no** error enums, sentinels, or error codes,
* **no** explicit range checks, and **no** min/max constants,
* **no** `#ifdef` / `switch` / enum parameters.

The complete conditional surface is 4 `if`/`while` conditions on two `int`s:
`x > 0 || y > 0`, `x == 1 && y == 4`, `x > 0`, `y == 0`, `x < 3`.

So the library has **no error-return surface at all**. The table below therefore
enumerates every *rejection-like* behaviour that does exist — i.e. every input
class for which `driver` produces **no output and returns immediately** (the
library's only form of "refusing to do work") — plus the generic FFI boundary
cases the task requires (out-of-range values, extreme/one-past-range values,
enum-shaped values). "Expected C result" is the observable behaviour: bytes
written to stdout and whether the call returns.

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `driver` | `x == 0 && y == 0` — loop guard `x>0 \|\| y>0` false on entry | returns immediately, **0 bytes** of output |
| 2  | `driver` | `x < 0 && y == 0` (e.g. `-1, 0`) — guard false | returns immediately, 0 bytes |
| 3  | `driver` | `x == 0 && y < 0` (e.g. `0, -1`) — guard false | returns immediately, 0 bytes |
| 4  | `driver` | `x < 0 && y < 0` (e.g. `-7, -3`) — guard false | returns immediately, 0 bytes |
| 5  | `driver` | `x == INT_MIN && y == 0` — most-negative int, guard false | returns immediately, 0 bytes |
| 6  | `driver` | `x == 0 && y == INT_MIN` — most-negative int, guard false | returns immediately, 0 bytes |
| 7  | `driver` | `x == INT_MIN && y == INT_MIN` — both extremes, guard false | returns immediately, 0 bytes |
| 8  | `driver` | `x == INT_MIN && y == -1` / `x == -1 && y == INT_MIN` — one step past the "valid" (>0) range in the negative direction | returns immediately, 0 bytes |
| 9  | `driver` | `x == 0`, `y > 0` — `x > 0` body branch never taken; `if (y == 0) continue` is the only exit route | terminates after `y` "y" lines, no "x" lines |
| 10 | `driver` | `x < 0`, `y > 0` (e.g. `INT_MIN, 3`) — negative `x` never decremented, `x < 3` always true | terminates, `"loop\n"` once then `y`×`"y\n"` |
| 11 | `driver` | `y == 0`, `x > 0` — `continue` taken on every iteration (the `y--` underflow path is never reached) | terminates after `x` iterations of `"loop\nx\n"` |
| 12 | `driver` | `x == 1 && y == 4` — the *only* input that takes `goto label2`, skipping the `label1` block | first iteration emits `"loop\n"` then `"y\n"` (no leading `"x\n"`) |
| 13 | `driver` | out-of-range "enum-shaped" ints passed across FFI: `x`/`y` set to values with no special meaning at all (`0x5A5A5A5A`, `-2`, `2`, `3`, `4`, `5`, `INT_MAX` guarded by the other arg being <= 0) | no validation exists; the same purely arithmetic behaviour, identical bytes |
| 14 | `driver` | **UB / non-terminating class (excluded from execution):** `x > 0 && y < 0`. `y` is decremented in `if (y == 0) continue;`-guarded code, but `y < 0` never equals 0, so `y--` runs forever until signed-integer overflow — undefined behaviour in C, and unbounded stdout output. | C: undefined behaviour / does not terminate. **Not executed** in the differential tests (any such test would hang or diverge legitimately). Documented and asserted structurally instead: the Rust uses `wrapping_sub`, so it cannot panic on overflow where C is UB. |
| 15 | `driver` | **Impractical (excluded from execution):** any input with `x.max(0) + y.max(0)` near `INT_MAX` — e.g. `(INT_MAX, 0)` terminates only after ~2^31 iterations, and `(0, INT_MAX)` after ~2^31 `"y\n"` lines. | Would match, but is not executable within the test budget (an early version of the suite actually hit `SIGXFSZ` here). The harness's `is_intractable()` refuses `x.max(0)+y.max(0) > 50_000`, and `configs_row17` asserts that these inputs are the ones being refused. Covered indirectly by large-but-tractable magnitudes (up to 20000) in `CONFIGS.md` rows 11-13. |

## Null pointers / lengths

The public API surface is `void driver(int, int)` — it accepts **no pointers,
no lengths, no buffers, no enums and no structs**. There is consequently no
null-pointer, zero-length or oversized-length case to construct; the "generic
boundaries every C API has" reduce to the integer extremes in rows 1-13, all of
which are tested.

## Status

All rows 1-13 have a passing differential test in
`translation/tests/differential.rs` (`errors_*` tests). Rows 14 and 15 are
non-executable by construction (C-side UB / ~2^32 lines of output) and are
justified above rather than executed.
