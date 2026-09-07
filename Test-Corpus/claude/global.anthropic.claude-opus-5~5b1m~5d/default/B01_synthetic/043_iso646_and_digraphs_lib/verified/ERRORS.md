# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/driver.c` + `c_src/include/driver.h`
(the entire C source, 33 + 29 lines). The C source is written with ISO 646
alternative spellings / digraphs (`%:` = `#`, `<%` = `{`, `%>` = `}`,
`bitor` = `|`, `compl` = `~`), i.e. it is equivalent to:

```c
void driver(int x, int y) {
    int result = x | ~y;
    printf("%d", result);
    puts("");
}
```

## Mechanical grep for rejection sites

```
grep -nE 'return|assert|NULL|errno|exit|abort|<|>|==|!=|RETURN_ERROR|if|switch|MIN|MAX' c_src/src/driver.c c_src/include/driver.h
```

Findings (excluding the license comment block and the include guard
`%:ifndef DRIVER_H_`):

* `return` statements: **none** (the function returns `void`).
* `assert` / `<assert.h>`: **none**.
* `NULL` checks / pointer parameters: **none** (both parameters are `int`
  by value; the function takes no pointers at all).
* explicit range checks, `if`, `switch`, `?:`: **none** — the body is
  straight-line code with no branches.
* min/max constants, error enums, error-return macros: **none**.
* library calls that can fail: `printf` and `puts` return `int`, but the C
  code **discards both return values**, so an I/O failure is not turned into
  any observable rejection.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | `driver` | *(none — the C code contains no rejection, error return, assertion, range check, or null check whatsoever)* | n/a |

The error surface of this library is **empty**: `driver` accepts every one of
the 2^64 possible `(int, int)` argument pairs and returns `void`
unconditionally. There is no error code, sentinel, or `errno` contract to
diverge on.

## Generic boundary coverage (required even though the table is empty)

Because the table has no rows, Phase C instead exhausts the generic
boundaries that *do* apply to this ABI surface. Each is a differential test
asserting C and Rust produce byte-identical stdout (and neither aborts):

| # | boundary class | concrete input(s) | test |
|---|----------------|-------------------|------|
| G1 | zero / identity values | `(0, 0)`, `(0, -1)`, `(-1, 0)`, `(-1, -1)` | `boundary_zero_and_identity` |
| G2 | extremal `int` values (one step past them is not representable in the ABI) | full cross product of `{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, INT_MAX-1, INT_MAX}` | `boundary_extremal_cross_product` |
| G3 | signed-overflow-adjacent operands: `~INT_MIN`, `~INT_MAX`, `x | ~y == INT_MIN` | `(INT_MIN, INT_MAX)`, `(INT_MAX, INT_MIN)`, `(INT_MIN, INT_MIN)`, `(INT_MIN, 2147483646)` | `boundary_overflow_adjacent` |
| G4 | "out-of-range enum" analogue: bit patterns with no meaningful interpretation, incl. values that are *not* valid small ints — every 32-bit pattern is a legal `int`, so this is covered by feeding raw random `u32` bit patterns reinterpreted as `int` | 20 000 random `(u32, u32)` pairs, fixed seed | `boundary_raw_bit_patterns` |
| G5 | truncation / widening at the FFI boundary: passing values that occupy the full 32 bits while the register is 64-bit (upper-half garbage must be ignored identically) | `(x, y)` where the caller's 64-bit register holds `0xDEADBEEF_00000000 \| x` | `boundary_register_upper_half` |
| G6 | no-pointer / no-null contract: confirm `driver` has no pointer parameter, so a null-pointer test is not constructible | documented, asserted by header inspection | `boundary_no_pointer_params` (documentation test) |

There are no out-of-range *enum* values to test because the API declares no
enum type; the closest constructible analogue (G4/G5, arbitrary raw bit
patterns and dirty upper register halves) is tested exhaustively-at-random
instead.
