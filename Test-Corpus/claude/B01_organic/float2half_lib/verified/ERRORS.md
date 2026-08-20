# ERRORS.md — Phase A error-surface table

Derived mechanically from `c_src/src/lib.c` (118 lines) and
`c_src/include/lib.h` (3 lines).

## Mechanical grep for every rejection construct

```sh
$ grep -nE 'return|assert|NULL|if *\(|switch|#if|errno|error|ERROR|exit|abort' \
      c_src/src/lib.c | grep -vE ':\s+0x'
117:    return (uint16_t)((uint32_t)m__base[j] + ((n & 0x007fffff) >> m__shift[j]));
```

| construct searched for | occurrences in C |
|------------------------|------------------|
| `return` statements | 1 (the single unconditional result on line 117) |
| `assert` / `static_assert` | 0 |
| `if` / `else` / `switch` / ternary `?:` | 0 |
| `#if` / `#ifdef` / `#ifndef` (conditional compilation) | 0 |
| error-return macros (`RETURN_ERROR`, `CHECK`, `GOTO_*`) | 0 |
| `return -1` / `return NULL` / error enums / error codes | 0 |
| pointer parameters (hence null checks) | 0 |
| length/count/size parameters (hence range checks) | 0 |
| `errno`, `exit`, `abort`, `longjmp` | 0 |
| min/max validation constants | 0 |
| heap allocation (hence allocation-failure paths) | 0 |

## ERROR-SURFACE TABLE

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | — | **none** | — |

**The table is empty, and that is a derived result, not an omission.**
`float2half` is a *total function*: it takes one `float` by value, returns one
`uint16_t` by value, and has no rejection path whatsoever. Every one of the
2^32 possible `float` bit patterns is a **valid** input that produces a defined
result, so there is no input the C "rejects". Justification per parameter and
per operation:

| aspect | why no error path exists |
|--------|--------------------------|
| parameter `float flt` | passed **by value**; not a pointer, so *no null-pointer path can exist* and no null test is possible to write. Every 32-bit pattern is a legal `float` object representation. |
| no length/size/count parameter | *no zero-length or oversized-length path can exist*; there is nothing to bound-check. |
| no enum parameter | *no out-of-range-enum path can exist*; there is no `enum`, `int` mode, or flag argument that could carry a value with no valid variant. |
| index `j = (n >> 23) & 0x1ff` | the `& 0x1ff` mask makes `j` provably `0..=511`, exactly the size of both 512-entry tables. **An out-of-bounds table read is unreachable for every input**, so no bounds check is needed and none exists. |
| shift `(n & 0x007fffff) >> m__shift[j]` | `m__shift` contains only the values 13..=24 (verified over all 512 entries), all `< 32`, so the shift of a `uint32_t` is never UB and never a Rust shift-overflow panic. |
| addition `(uint32_t)m__base[j] + (mant >> shift)` | max reachable sum is `0xFC00 + 0x3FF == 0xFFFF`, so the truncating cast to `uint16_t` never actually discards a bit; no overflow path. |
| return value | one unconditional expression; there is no sentinel or error value in the `uint16_t` range — every returned bit pattern is a legitimate result. |

## Generic-boundary coverage still required by Phase C

Because the table is empty, Phase C instead pins down the *degenerate and
boundary inputs* that a caller can actually supply across the FFI boundary, and
asserts C and Rust agree bit-for-bit (a divergence here would be the
error-surface equivalent). Tested in `tests/error_paths.rs`:

| # | condition | why it is the boundary | expected |
|---|-----------|------------------------|----------|
| G1 | `+0.0` (`0x00000000`) and `-0.0` (`0x80000000`) | zero / signed-zero degenerate input | `0x0000` / `0x8000`; sign preserved |
| G2 | smallest and largest positive **subnormal** f32 (`0x00000001`, `0x007FFFFF`) and negative equivalents | underflow boundary, `exp==0` | flush to `0x0000` / `0x8000` |
| G3 | `+Inf` (`0x7F800000`), `-Inf` (`0xFF800000`) | non-finite input | `0x7C00` / `0xFC00` |
| G4 | quiet NaN (`0x7FC00000`, `0xFFC00000`) | non-finite, mantissa nonzero | `0x7E00` / `0xFE00` (payload shifted in) |
| G5 | **signalling** NaN (`0x7F800001`, `0xFF800001`) and all-ones NaN (`0x7FFFFFFF`, `0xFFFFFFFF`) | sNaN must NOT be quieted while crossing the FFI boundary | bit-exact match; Rust must not canonicalise the payload |
| G6 | one step *past* the half-overflow threshold: `j=142` &rarr; `j=143` (`0x47000000`, `0x47800000`, `0x477FFFFF`) | one step past the last representable-as-finite exponent | last finite half vs. `0x7C00` |
| G7 | one step *before/after* the half-subnormal threshold: `j=102` &rarr; `j=103` (`0x33FFFFFF`, `0x34000000`) | one step past the flush-to-zero range | `0x0000` vs. `0x0001` |
| G8 | one step *past* the smallest half normal: `j=112` &rarr; `j=113` (`0x387FFFFF`, `0x38800000`) | subnormal/normal half boundary | `0x03FF` vs. `0x0400` |
| G9 | every `j` boundary pair `(j, j+1)` for all 511 pairs, plus `j=0` and `j=511` extremes | exhaustive "one step past the valid range" for the *only* index the code derives | bit-exact match |
| G10 | all 512 `j` values &times; mantissa `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` | max/min of the only sub-field the code reads | bit-exact match |
| G11 | **all 2^32 bit patterns** (exhaustive) | leaves no reachable input untested | bit-exact match |
