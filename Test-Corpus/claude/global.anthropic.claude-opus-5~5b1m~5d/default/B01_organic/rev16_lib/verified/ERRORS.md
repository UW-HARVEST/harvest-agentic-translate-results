# ERRORS.md — Error-surface table

## Derivation

Mechanically grepped the entire C tree (`c_src/include`, `c_src/src`) for every
rejection construct:

```
grep -rnE 'return|assert|NULL|errno|-1|if|switch|#ifdef|#if|goto|abort|exit|<|>=|<=|==|!=|MIN|MAX|ERROR|enum' include src
```

Every hit was either one of the four bit-twiddling statements (matched only on
the `<<` / `>>` shift operators) or the single success statement `return a;`.

Findings:

* **`return` statements:** 1 total — `return a;` in `rev16`, the success path.
  There is no `return -1`, no `return NULL`, no error-return macro
  (`RETURN_ERROR` or similar), and no error enum anywhere in `c_src/`.
* **`assert` / `abort` / `exit`:** 0 occurrences. `<assert.h>` is not included.
* **`if` / `switch` / `goto` / ternary:** 0 occurrences. `rev16` is fully
  branch-free straight-line code.
* **`#if` / `#ifdef` / `#ifndef`:** 0 occurrences. No conditional compilation.
* **Null checks:** not applicable — the API takes no pointers. `rev16`'s only
  parameter is a `uint32_t` passed by value, and it returns `uint32_t` by value.
* **Range checks / min-max constants:** 0 occurrences. No input value is
  rejected; the only constants are the bit masks `0xAAAA`, `0x5555`, `0xCCCC`,
  `0x3333`, `0xF0F0`, `0x0F0F`, `0xFF00`, `0x00FF`, which are operands of `&`,
  not bounds.
* **Enums across the FFI boundary:** none exist; the sole parameter is an
  integer with no invalid values.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | — | *(none — the C library has no rejection path)* | — |

**The table is empty by derivation, not by omission.** `rev16` is a total
function over its entire `uint32_t` domain: all 2^32 inputs are valid, none is
rejected, and it always returns normally.

## Consequence for Phase C

Because there is no error code or sentinel to compare, the Phase C obligation
degenerates into proving *total-ness parity*: for the input classes that would
be "invalid" in a typical C API, C and Rust must both accept the input and
return the same value rather than trapping. `tests/differential.rs` covers this
explicitly in `phase_c_*`:

| # | boundary class (generic, per the Phase C instructions) | test |
|---|--------------------------------------------------------|------|
| C1 | zero input (`a == 0`) | `phase_c_zero` |
| C2 | maximum / "oversized" input (`a == u32::MAX`), i.e. every bit set including the bits the masks discard | `phase_c_saturated_and_extremes` |
| C3 | one step past the 16-bit range the masks operate on (`0x10000`, `0xFFFF + 1`, `0xFFFF - 1`, `0x10001`) | `phase_c_one_past_range` |
| C4 | inputs whose entire payload lives in the discarded high half (`0xFFFF0000`, `0x8000_0000`, every single high bit) | `phase_c_high_half_only` |
| C5 | every single-bit input, bits 0..=31 (includes each out-of-mask bit individually) | `phase_c_single_bits` |
| C6 | out-of-range enum values across FFI | not applicable — no enum parameter exists (documented above) |
| C7 | null pointers | not applicable — no pointer parameter exists (documented above) |

`phase_c_exhaustive_low_17_bits` additionally proves the parity over the whole
`0..=0x1FFFF` range, which contains every boundary in C1–C5 by construction.
