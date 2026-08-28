# ERRORS.md — Phase A error-surface table

Mechanically derived by grepping the entire C source for every rejection
mechanism. The search covered:

```
grep -nE "RETURN_ERROR|return -1|return NULL|assert|errno|if *\(|switch|\
#ifdef|#if |exit\(|abort\(|NULL" c_src/src/lib.c c_src/include/lib.h
```

**Result: zero matches.** `c_src/src/lib.c` contains no `if`, no `switch`, no
`assert`, no `return NULL`, no error enum, no sentinel return, no `errno` use,
no range check, no null check, and no preprocessor conditional. There is no
`#define`d min/max constant.

## Why the error surface is genuinely empty (not merely un-grepped)

`half2float` is a **total function** over its entire declared domain. The
argument is `uint16_t`, so all 65 536 possible inputs are valid and every one
of them takes the exact same straight-line path:

```c
int n = h >> 10;                                              /* n ∈ 0..=63  */
out.num = m__mantissa[(h & 0x3ff) + m__offset[n]] + m__exponent[n];
return out.flt;
```

The two index expressions are provably in bounds for every input:

| expression | range over all `uint16_t h` | table size | in bounds? |
|------------|------------------------------|------------|-----------|
| `n = h >> 10` | `0 .. 63` (a 16-bit value shifted right 10 keeps 6 bits) | `m__offset[64]`, `m__exponent[64]` | yes, always |
| `(h & 0x3ff) + m__offset[n]` | `0 .. 1023` + `{0x0000, 0x0400}` = `0 .. 2047` | `m__mantissa[2048]` | yes, always |

So there is no input the C can reject, and therefore no error code, sentinel or
crash to match. There are **no rows** in the error-surface table:

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | — | *(none — the C source contains no rejection path)* | — |

## Generic boundary cases still tested (Phase C)

Even with an empty table, Phase C exercises the generic boundaries that every C
API has, so that the *absence* of an error path is itself verified to be
faithfully reproduced rather than assumed. Each is a differential test asserting
C and Rust return **bit-identical** results:

| # | boundary class | concrete input(s) | status |
|---|----------------|-------------------|--------|
| B1 | minimum value of the domain | `h = 0x0000` (+0.0) | [x] |
| B2 | maximum value of the domain | `h = 0xFFFF` (negative NaN) | [x] |
| B3 | every value one step past each internal region edge | `h ∈ {0x03FF, 0x0400, 0x7BFF, 0x7C00, 0x7C01, 0x7FFF, 0x8000, 0x83FF, 0x8400, 0xFBFF, 0xFC00, 0xFC01}` | [x] |
| B4 | **exhaustive**: the entire input domain | all 65 536 values of `h`, compared bit-for-bit | [x] |
| B5 | out-of-range value passed across the FFI boundary — the analogue of an invalid enum: caller declares the callee as taking a *wider* integer (`uint32_t`/`uint64_t`) and passes a value whose high bits are set, so the incoming register holds bits outside `uint16_t` | `0x1_0000`, `0xDEAD_0000 \| h`, `0xFFFF_FFFF`, and 64-bit `0xFFFF_FFFF_FFFF_0000 \| h` | [x] |
| B6 | no pointer arguments exist, so there is no null-pointer or length boundary to test | n/a — signature is `float(uint16_t)`, by inspection of `lib.h` | n/a |

Note on B5: the x86-64 SysV ABI leaves the high bits of a register holding a
narrow argument unspecified, so a caller that lies about the prototype is
outside the contract. The test pins down that C and Rust nevertheless agree,
i.e. both narrow the incoming register to 16 bits identically, so no divergence
is reachable even from a mis-declared caller.

Note on B4: because the domain is only 65 536 values wide, Phase C's exhaustive
sweep is a *complete* proof of behavioural equivalence — it leaves no untested
input, so no error path can hide in an unvisited corner of the domain.
