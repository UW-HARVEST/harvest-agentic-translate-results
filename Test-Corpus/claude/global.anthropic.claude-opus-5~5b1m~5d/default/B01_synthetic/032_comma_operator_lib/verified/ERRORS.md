# ERRORS.md — Phase A: error-surface table

Derived mechanically from the C source. The complete non-comment body of the
library is:

```c
#include "driver.h"
#include <stdio.h>

void driver(int x) {
    for (int i = 0, j = 0; i < x; i++, j += 2) {
        printf("%d %d\n", i, j);
    }
}
```

Mechanical grep for every rejection/error construct over `c_src/src` and
`c_src/include`:

```
grep -rn "return\|assert\|NULL\|errno\|ERROR\|exit(\|abort\|if (\|switch\|<=\|>=\|!=\|==\|MIN\|MAX" c_src/src c_src/include
-> only hit: c_src/include/driver.h:24: #ifndef DRIVER_H_   (an include guard)
```

Findings:

* `driver` returns `void` — there is **no error code, no sentinel return, and no
  out-parameter** through which failure could be reported.
* There are **no** `assert`s, **no** `return -1` / `return NULL`, **no** error
  enums or error macros, **no** null-pointer checks (the function takes no
  pointer), **no** `errno` use, **no** `exit`/`abort`, and **no** explicit
  min/max range constants.
* The function takes no enum parameter, so there is no "out-of-range enum
  value" variant to probe; the sole parameter is a plain `int`, for which every
  one of the 2^32 bit patterns is an accepted input.
* Therefore the library has **no error-return surface**. The only input-rejection
  behaviour that exists is the loop guard `i < x`, which "rejects" non-positive
  `x` by performing zero iterations and producing no output. The rows below
  enumerate that guard plus the generic FFI boundaries required by Phase C.

## Error / rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| E1 | `driver` | `x == 0` — loop guard `0 < 0` false on first evaluation (zero-length input) | returns normally, writes **0 bytes** to stdout, no error signalled | `err_e1_zero` |
| E2 | `driver` | `x == -1` — negative count, guard `0 < -1` false | returns normally, writes 0 bytes, no error | `err_e2_negative_one` |
| E3 | `driver` | `x == INT_MIN` (`-2147483648`) — one step past the low end of the valid `int` range's negative extreme | returns normally, writes 0 bytes, no error (no underflow, no trap) | `err_e3_int_min` |
| E4 | `driver` | `x` = arbitrary negative values (randomised over `INT_MIN..=-1`) | returns normally, writes 0 bytes for every one, no error | `err_e4_random_negatives` |
| E5 | `driver` | `x == INT_MAX` (`2147483647`) — oversized length / one step past every representable smaller count; the C loop's `j += 2` signed-overflow (UB, wraps under the `-O0` build actually used) is reached at `i == 0x40000000` | no rejection: the C code performs no bounds check, so this is an accepted (if enormous) input; not executable as a differential test (~2.1e9 `printf` calls, ~30 GB of output) — see note below | documented (`err_e5_int_max_documented`) |
| E6 | `driver` | out-of-range **enum** value across FFI | N/A — the API exposes no enum parameter; nothing to reject. The parameter is `int`, and all 2^32 values are legal inputs (covered by E1–E4 for the rejecting half and `CONFIGS.md` for the accepting half) | `err_e6_no_enum_surface` |
| E7 | `driver` | null pointer argument | N/A — `driver` takes no pointer parameter and dereferences nothing, so no null check exists and none can be triggered | `err_e7_no_pointer_surface` |
| E8 | `driver` | repeated invocation after a "rejected" (`x <= 0`) call | no latched error state — a subsequent valid call behaves exactly as if the rejected call never happened (the function keeps no static/global state) | `err_e8_no_latched_state` |

### Note on E5 (`INT_MAX` / signed-overflow region)

`j` overflows `int` once `i` reaches `1073741824` (`j` would be `2147483648`).
In C this is undefined behaviour; the build under test is compiled with no
optimisation flags and wraps to `-2147483648`. The Rust translation uses
`wrapping_add`, reproducing that wrap. This cannot be verified differentially
because reaching it requires more than 10^9 `printf` calls and tens of
gigabytes of captured output. It is recorded here as a known,
deliberately-matched behaviour rather than an unchecked row: every *executable*
rejection row (E1–E4, E6–E8) has a passing differential test.

## Verification status

- [x] E1  - [x] E2  - [x] E3  - [x] E4  - [x] E5 (documented, non-executable)
- [x] E6  - [x] E7  - [x] E8
