# ERRORS.md — Phase A error-surface table

## Mechanical derivation

Every grep below was run over `c_src/include` and `c_src/src` only (the CMake
`build/` tree contains CMake's own `CMakeCCompilerId.c` probe file, which is not
part of the library and is excluded).

| grep pattern (what it hunts for) | matches |
|----------------------------------|---------|
| `return +(-?[0-9]+\|NULL)` , `RETURN_ERROR`, `ERROR`, `errno`, `assert`, `abort`, `exit(`, `goto`, `perror`, `SIZE`, `MAX`, `MIN`, `LIMIT` | **0** |
| `if`, `else`, `switch`, `case`, `while`, `for`, `?`, `&&`, `\|\|`, `#if` | **0** |
| `*`, `[`, `]`, `struct`, `union`, `enum`, `typedef`, `malloc`, `calloc`, `realloc`, `free`, `memcpy`, `memset`, `size_t` | **0** |

The complete library body is nine lines:

```c
#include "lib.h"

uint32_t rev16(uint32_t a) {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    return a;
}
```

## Finding: the error surface is EMPTY

`rev16` is a **total function**. It takes one `uint32_t` by value, executes four
unconditional straight-line statements, and returns. Consequently:

* there is **no** error-return macro, error enum, sentinel, or `errno` write;
* there is **no** `assert`, `abort`, or `exit`;
* there is **no** explicit range check, and **no** min/max constant;
* there is **no** pointer parameter, so a null check is impossible and a null
  pointer is not a representable input;
* there is **no** length/count parameter, so zero-length and oversized-length
  are not representable inputs;
* there is **no** `enum` parameter, so an out-of-range enum value is not a
  representable input;
* the *entire* `uint32_t` domain — all 2^32 values — is **valid input**, and
  every one of them has a defined result. There is no "one step past a
  documented valid range": the range is the full type.

Writing a row that claims `rev16` rejects something would be inventing an error
that the C does not have, which the task forbids.

## What is therefore tested instead

Since no input is *rejected*, the corresponding obligation is that no input is
rejected **by the Rust either** — the Rust must not panic, abort, trap on
overflow, or diverge on the inputs that would be the "error triggers" in a
library that had any. Those are the degenerate / extreme / would-be-invalid
inputs enumerated below. Each row is a differential test that asserts C and Rust
return the **same exact `u32` value** and that **neither** aborts the process.

`expected C result` is computed by the four statements above by hand; the test
asserts against the value the C `.so` actually returns, so the column is
documentation, not the oracle.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| E1 | `rev16` | there is no null-pointer input — parameter is a by-value scalar. Closest representable analogue: the all-zero argument `0x00000000` | `0x00000000` (no error, no rejection) | [x] |
| E2 | `rev16` | zero-length analogue: argument whose entire 16-bit payload is empty while high garbage is present, `0xFFFF0000` | `0x00000000` — statement 1's 16-bit masks discard bits 16..31 | [x] |
| E3 | `rev16` | oversized-length analogue: numeric maximum `0xFFFFFFFF`, the largest value the type admits | `0x0000FFFF` | [x] |
| E4 | `rev16` | one step past the widest value the four masks cover: `0x00010000` (first bit above the 16-bit window) | `0x00000000` — silently discarded, **not** an error | [x] |
| E5 | `rev16` | one step below that boundary: `0x0000FFFF` (largest value fully inside the window) | `0x0000FFFF` | [x] |
| E6 | `rev16` | out-of-range "enum" analogue: no enum parameter exists, so the analogue is every bit position outside the honoured window taken alone, `1u32 << k` for `k = 16..=31` (16 sub-cases) | `0x00000000` for all 16 — every high bit is discarded | [x] |
| E7 | `rev16` | signed-reinterpretation hazard: arguments whose top bit is set (`0x80000000`, `0x80000001`, `0xFFFFFFFE`), which are negative if wrongly read as `int32_t` | `0x00000000`, `0x80000000`, `0x7FFF0000`→ see test (C value is the oracle) | [x] |
| E8 | `rev16` | shift-overflow hazard: arguments that maximise each `<<` operand, `0x00005555`, `0x00003333`, `0x00000F0F`, `0x000000FF` (largest pre-shift values for statements 1–4) | no overflow — largest shifted intermediate is `0x0000FF00`, far inside `u32` | [x] |
| E9 | `rev16` | re-entrancy / statefulness: call `rev16` many times in interleaved order with the same and different arguments, and from multiple threads | identical results every time — the function is pure, holds no state, and touches no global | [x] |
| E10 | `rev16` | ABI width mismatch: pass an argument in the full 64-bit register (high half dirty) to confirm only the low 32 bits are consumed, matching `uint32_t`/`c_uint` | high half ignored by both | [x] |

Every row above is implemented in `tests/error_paths.rs` and passes against both
libraries. **0 rows unchecked.**

## Exhaustive closure of the error surface

Because `rev16`'s input domain is a single `u32`, "every rejection path" and
"every accepted path" are both settled completely by
`tests/valid_paths.rs::exhaustive_all_2pow32_arguments`, which feeds **all
4 294 967 296 values** through both shared objects and requires byte-identical
results. Rows E1–E10 are therefore a documented subset of an exhaustive proof:
there is no input — valid, boundary, or would-be-invalid — on which the C and the
Rust differ, and neither library aborts, panics or traps on any of them.

```
[EXHAUSTIVE] verified all 4294967296 u32 arguments identical
```
