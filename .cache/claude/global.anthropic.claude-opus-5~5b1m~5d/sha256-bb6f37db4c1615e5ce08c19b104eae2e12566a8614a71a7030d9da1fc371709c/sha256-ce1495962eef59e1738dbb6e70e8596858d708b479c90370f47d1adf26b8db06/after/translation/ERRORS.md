# ERRORS.md — error-surface table (Phase A / Phase C)

Derived mechanically from the C source. The complete C implementation is:

```c
void driver(int x) {
    auto int y = 2*x;
    y += 300;
    printf("%d\n", y);
}
```

## Mechanical grep for rejection sites

```sh
grep -nE 'return|assert|RETURN|NULL|if|switch|errno|-1|<|>|\?' \
     c_src/src/driver.c c_src/include/driver.h
```

Hits (excluding comments): only `#include <stdio.h>` and the `DRIVER_H_` include
guard. Specifically the C source contains:

* **0** `return` statements (the function is `void` and falls off the end)
* **0** error-return macros (`RETURN_ERROR`, `return -1`, `return NULL`, …)
* **0** `assert` / `abort` / `exit` calls
* **0** `if` / `switch` / ternary branches
* **0** range checks, null checks, length checks
* **0** min/max constants, **0** error enums, **0** `errno` use
* **0** pointer parameters and **0** enum parameters

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| — | `driver` | *(none — the C code contains no rejection, error-return, assert, or range-check site whatsoever)* | n/a |

The table is **empty by construction**: `driver` is total over its domain. Every
one of the 2^32 values of an `int` argument is accepted and produces output. There
is no error code, no sentinel return value (the function returns `void`), and no
out-of-band failure signal to compare.

## Generic-boundary rows tested anyway (Phase C obligations)

Because "no error surface" must be *verified* rather than assumed, the following
generic C-API boundary conditions are still exercised differentially. Rows marked
N/A-by-signature are recorded so the absence is explicit and auditable.

| # | boundary class | applies? | what is tested / why not | status |
|---|----------------|----------|--------------------------|--------|
| G1 | null pointer arguments | **N/A** | `driver`'s only parameter is `int` by value; there is no pointer in the public ABI, so no null-pointer input exists. | N/A ✅ |
| G2 | zero length / empty input | **N/A** | no length, size, or count parameter exists. | N/A ✅ |
| G3 | oversized length | **N/A** | no length parameter exists. | N/A ✅ |
| G4 | out-of-range enum value across FFI | **N/A** | no enum parameter exists; `int` has no invalid bit pattern. | N/A ✅ |
| G5 | zero value | yes | `driver(0)` → must print the same bytes from C and Rust. | [x] tested (`boundary_and_error_surface`) |
| G6 | one step past representable maximum | yes | `INT_MAX` and `INT_MAX - 1`: `2*x` overflows signed `int`. C (unoptimised, x86-64 `imul`) wraps two's-complement; Rust uses `wrapping_mul`. Must match. | [x] tested (`boundary_and_error_surface`) |
| G7 | one step past representable minimum | yes | `INT_MIN` and `INT_MIN + 1`: `2*x` overflows negatively. Must match. | [x] tested (`boundary_and_error_surface`) |
| G8 | overflow introduced by the `+= 300` step only | yes | `x = (INT_MAX-300)/2 = 1073741673` (no overflow) and `x = 1073741674` (the add overflows). The two sides of that exact boundary must match. | [x] tested (`boundary_and_error_surface`) |
| G9 | overflow boundary of the `2*x` step | yes | `x = INT_MAX/2 = 1073741823`, `1073741824`, `INT_MIN/2 = -1073741824`, `-1073741825`. | [x] tested (`boundary_and_error_surface`) |
| G10 | value making output exactly zero / sign flip | yes | `x = -150` → `0`; `x = -149`/`-151` → `+2`/`-2`. Exercises the `%d` sign path. | [x] tested (`boundary_and_error_surface`) |
| G11 | no crash / no panic on any input | yes | The Rust `.so` must not panic (debug builds enable overflow checks; the translation uses `wrapping_*`). Verified by the fact that every differential call returns normally across 20 000+ randomized full-range inputs. | [x] tested (`randomized_full_range`) |

**Every row above is either N/A-by-signature (justified) or covered by a passing
differential test.** No row is unchecked.
