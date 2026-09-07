# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every rejection path was searched for in `c_src/src/lib.c`. Because 343 of the
file's 376 lines are pure table data, the grep excluded lines consisting only of
hex literals, leaving the complete set of executable lines:

```
$ grep -vnE '^\s+0x|^\s*0x' c_src/src/lib.c
1:#include "lib.h"
3:static uint32_t m__mantissa[2048] = {
346:static uint16_t m__offset[64] = {
355:static uint32_t m__exponent[64] = {
368:float half2float(uint16_t h) {
369:    union {
370:        float flt;
371:        uint32_t num;
372:    } out;
373:    int n = h >> 10;
374:    out.num = m__mantissa[(h & 0x3ff) + m__offset[n]] + m__exponent[n];
375:    return out.flt;
376:}
```

Searched-for patterns and their results:

| pattern searched | occurrences in executable code |
|---|---|
| `return -1`, `return NULL`, `return 0;` | 0 |
| `RETURN_ERROR` / error macros | 0 |
| `assert` / `abort` / `exit(` | 0 |
| `errno` | 0 |
| explicit range check (`if`, `switch`, `?:`, `<`, `>`, `==`, `!=`) | 0 |
| null-pointer check | 0 (the API takes no pointers) |
| `goto` / error labels | 0 |
| error enum / status type | 0 (return type is plain `float`) |
| `#ifdef` / `#if` conditional compilation | 0 |
| min/max validity constant | 0 |

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | — | *(no rows)* | — |

**The error surface is empty, and this is a derived fact, not an assumption.**
`half2float` is a total function over its domain. It takes a single `uint16_t`
by value, performs no allocation, dereferences no pointer, and contains zero
branches. It has no error return channel: the return type is `float`, with no
out-parameter and no sentinel. Every one of the 65 536 possible `uint16_t`
values is a *valid* input that produces a defined result, so there is no
"invalid input" for which C and Rust could disagree on a rejection.

Both table indices are provably in bounds for the whole domain, which is why no
range check exists in the C:

- `n = h >> 10` with `h` 16-bit ⇒ `n ∈ [0, 63]`, and both `m__offset` and
  `m__exponent` have exactly 64 elements.
- `(h & 0x3ff) + m__offset[n]` ⇒ at most `0x3ff + 0x400 = 0x7ff = 2047`, and
  `m__mantissa` has exactly 2048 elements.

## Boundary conditions tested anyway (Phase C)

Although the table has no rows, the generic boundaries every C API has are still
covered by `tests/differential.rs`, since "no error path" is itself a claim that
must be verified differentially rather than trusted:

| # | boundary | test |
|---|----------|------|
| B1 | minimum input `h = 0x0000` | `test_boundary_values` |
| B2 | maximum input `h = 0xFFFF` | `test_boundary_values` |
| B3 | one step past each sub-range boundary (`0x03FF/0x0400`, `0x7BFF/0x7C00/0x7C01`, `0x7FFF/0x8000`, `0xFBFF/0xFC00`) | `test_boundary_values` |
| B4 | **out-of-range value across the FFI boundary**: an argument wider than `uint16_t` with dirty high bits (`0x1_0000`, `0xDEAD_0000 \| h`, `0xFFFF_FFFF`) passed into the 16-bit parameter slot. C accepts any int in a narrow parameter slot, so this is a real input; both sides must truncate identically. | `test_ffi_dirty_upper_bits` |
| B5 | exhaustive: all 65 536 inputs, i.e. the entire domain, leaving no untested input for which a hidden rejection could exist | `test_exhaustive_all_inputs` |
| B6 | NaN payload preservation (bit-exact, not `==`, since `NaN != NaN`) | `test_exhaustive_all_inputs` compares raw bits via `to_bits()` |

There is no "oversized length" or "null pointer" boundary to test: the API has
no length parameter and no pointer parameter.

## Divergence found and fixed (boundary B4)

Boundary B4 caught a real defect. It is recorded here because it is the only
divergence the whole verification found, and it is exactly the class the
happy-path rows cannot see.

**Symptom.** `test_ffi_dirty_upper_bits` aborted the Rust `.so` with
`index out of bounds: the len is 2048 but the index is 29299`, SIGABRT (the
release profile sets `panic = "abort"`), where C returned a value.

**Root cause**, from the disassembly of both sides:

- C narrows its own parameter. `gcc` emits `movzwl -0x14(%rbp),%eax` at the top
  of `half2float`, so the callee reads only bits 0..15 of `%edi`. `n` is
  therefore always in `[0, 63]` no matter what the caller left in the register.
- The original Rust took `h: c_ushort`. That gives the LLVM parameter a
  `zeroext` attribute, i.e. "the caller already zero-extended", so LLVM shifted
  the *full* 32-bit register (`mov %edi,%ecx; shr $0xa,%ecx`) and, having proved
  `n < 64` from the `u16` type, **deleted the `M__OFFSET` bounds check
  entirely** — an unchecked out-of-bounds read. The surviving `M__MANTISSA`
  check then aborted.

So there were two defects with no C counterpart: a silent OOB read on
`M__OFFSET`/`M__EXPONENT`, and an abort where C returns.

**First attempt was insufficient.** Writing `let n = ((h >> 10) & 0x3f)` while
keeping the `u16` parameter did remove the mantissa panic, but LLVM *folded the
`& 0x3f` away* — the disassembly still showed a bare `shr $0xa,%edi` — because
the `zeroext` assumption makes the mask provably redundant. The OOB reads
remained. Masking cannot fix this from behind a `u16` parameter.

**Fix.** Declare the exported wrapper as taking the full argument slot
(`c_uint`) and truncate explicitly, then call an inner `half2float_impl(u16)`
that is the literal translation:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn half2float(h: c_uint) -> c_float {
    half2float_impl((h & 0xffff) as u16)
}
```

On x86-64 SysV a `uint16_t` argument occupies `%edi`, the same slot a `c_uint`
occupies, so the ABI footprint is unchanged and conforming callers are
unaffected (the mask is a no-op for them). The masks are now preserved —
`and $0x3f,%edi` and `and $0x7ff,%ecx` both survive — both indices are provably
in bounds, and the emitted code is straight-line to `ret` with no panic path at
all, matching C's total behavior for every possible register value.

Equivalence for an arbitrary 32-bit slot value `a`:

| quantity | C | Rust after fix |
|---|---|---|
| `n` | `(a & 0xffff) >> 10` = bits 10..15 | `((a & 0xffff) >> 10) & 0x3f` = bits 10..15 |
| `m` | `(a & 0xffff) & 0x3ff` = bits 0..9 | `(a & 0xffff) & 0x3ff` = bits 0..9 |

## Suite validity (mutation testing)

A passing suite is only meaningful if it can fail, so the harness was checked
against deliberately broken Rust. Each mutation was applied, the suite run, and
the source then regenerated from the C by `tools/gen_lib_rs.py` and re-diffed.

| mutation | detected? | by |
|---|---|---|
| single bit flipped in one `M__MANTISSA` entry (`m__mantissa[1]`, reachable only at `h = 0x0001`) | yes — 8 tests failed | rows 2, 10, 17, 19, 20 |
| FFI boundary truncation removed (back to a plain `u16` parameter) | yes — SIGABRT | `test_ffi_dirty_upper_bits` |

The first mutation matters as evidence that `test_row19_exhaustive_all_inputs`
really walks the whole domain: the corrupted entry is reachable from exactly one
input, `h = 0x0001`, and row 19 flagged it.
