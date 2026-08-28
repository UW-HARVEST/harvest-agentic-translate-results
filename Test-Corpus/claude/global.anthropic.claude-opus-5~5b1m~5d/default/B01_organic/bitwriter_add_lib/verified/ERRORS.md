# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. Exhaustive greps over the whole C
source tree:

```
grep -n 'return'                          -> line 23 only: `return 0;`
grep -n -E 'assert|NULL|ERROR|errno|if \(|switch|#ifdef|#if '  -> NO MATCHES
grep -n -E 'while|\?|else'                -> line 11 (while), line 13 (ternary)
```

**Key finding — the C function has NO error surface.** There is not one
error-return macro, not one `return -1`, not one `return NULL`, no error enum, no
`assert`, no null check, no range check, and no min/max constant. `bitwriter_add`
has a single exit, `return 0`, which is unconditional. The local `int r;` on line 7
is declared and never assigned or returned.

Consequently the differential obligation on the error surface is the *inverse* of
the usual one: for every input a normal API would reject, the C **accepts** it,
executes C-undefined-behaviour shifts / unsigned wraparound, and still returns `0`.
The Rust must reproduce the same accepted-and-mangled state and the same `0`.
Every row below asserts *both* the return value and the full post-call struct.

The reference C is compiled by `CMakeLists.txt` with no `CMAKE_BUILD_TYPE`, i.e.
`-O0`. The emitted code (verified via `objdump -d`) uses `shlq %cl` / `shrq %cl`
with 64-bit operands, so every out-of-range shift count is reduced **mod 64** by
the hardware, and every `unsigned int` expression wraps **mod 2^32**. That is the
exact contract the Rust reproduces with `wrapping_shl` / `wrapping_shr` /
`wrapping_add` / `wrapping_sub`.

## Error / rejection table

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|---------------------------------------------|-------------------|-----|
| E1 | `bitwriter_add` | `bw == NULL` — pointer dereferenced at line 9 (`bw->tot += bits`) with no null check | no error code: process fatal `SIGSEGV` (signal 11). Rust must also fault, same signal, not return | [x] |
| E2 | `bitwriter_add` | `bits == 0` → line 8 shift count `64 - 0 == 64`, out of range for a 64-bit shift (C UB) | accepted, returns `0`; `%cl`-masked count `64 & 63 == 0` so `val` is left unshifted | [x] |
| E3 | `bitwriter_add` | `bits == 64` — boundary, exactly the operand width; `64 - 64 == 0` | accepted, returns `0`; shift by 0 | [x] |
| E4 | `bitwriter_add` | `bits == 65` — one step past the maximum meaningful width; `64 - 65` wraps to `0xFFFFFFFF`, shift count mod 64 == 63 | accepted, returns `0`, no rejection | [x] |
| E5 | `bitwriter_add` | `bits` huge / oversized length (`0x80000000`, `u32::MAX`) — no upper-bound check exists | accepted, returns `0`; wrapped shift + wrapped `bw->tot` | [x] |
| E6 | `bitwriter_add` | `bw->bits == 64` (invalid internal state ≥ operand width) → line 12 `64 - 64 - 1` wraps to `0xFFFFFFFF`, and line 14/21 `val >> bw->bits` is an out-of-range shift | accepted, returns `0`; shift counts masked mod 64 | [x] |
| E7 | `bitwriter_add` | `bw->bits > 64` up to `u32::MAX` (grossly invalid internal state), never validated | accepted, returns `0` | [x] |
| E8 | `bitwriter_add` | `bw->bits == 63` with `bits >= 1` → line 12/13 make `b == 0`, so the loop makes **no progress**. This is the one and only defensive construct in the C: the `i < 100` guard on line 11 | no error code: loop bails out after exactly 100 iterations and returns `0`. `bw->val` has had `mask` applied 100× and `bw->bits` is unchanged | [x] |
| E9 | `bitwriter_add` | `bits == 0` while `bw->bits >= 64` → loop entered (`bits+bw->bits >= 64`) but `b = min(b, 0) == 0`, again no progress | returns `0` after the 100-iteration cap | [x] |
| E10 | `bitwriter_add` | `bw->bits + bits` overflows `unsigned int` (e.g. `bw->bits = 0xFFFFFFFF`, `bits = 1` → sum `0`) — line 11 compares the **wrapped 32-bit** sum against 64 | accepted, returns `0`; loop is **not** entered at all despite both operands being huge | [x] |
| E11 | `bitwriter_add` | `bw->tot` overflow: `bw->tot = 0xFFFFFFFF`, `bits >= 1` → line 9 wraps mod 2^32, unchecked | accepted, returns `0`, `tot` wraps silently | [x] |
| E12 | `bitwriter_add` | out-of-range "enum"-style ints across FFI: `bits` is the only scalar selector and is `tflac_u32`; every one of the 2^32 values is a legal C input (no variant set). Sampled at `0,1,63,64,65,127,128,255,256,1000,0x7FFFFFFF,0x80000000,0xFFFFFFFE,0xFFFFFFFF` plus randomized | all accepted, all return `0` | [x] |
| E13 | `bitwriter_add` | fields the C never reads or writes (`pos`, `len`, `buffer`) set to garbage / dangling / non-null junk pointers | ignored entirely; must be byte-identical after the call, and must not be dereferenced | [x] |

## Notes on E1

`bw == NULL` cannot be tested in-process without killing the test runner, so the
Phase C test `fork()`s and compares the child's termination signal for the C `.so`
and the Rust `.so`. Both must die by the same signal (`SIGSEGV`) rather than
returning a value.
