# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c` (376 lines) and
`c_src/include/lib.h` (3 lines).

## Mechanical grep results

```
$ grep -nE 'RETURN_ERROR|return -1|return NULL|assert|errno|abort|exit\(|if *\(|switch|\?:' c_src/src/lib.c
(no matches)
```

The entire C source contains exactly **one** `return` statement
(`return out.flt;` at line 375) and **zero** branches, asserts, error macros,
error enums, null checks, range checks, or sentinel returns.

## The complete error surface

`half2float` has a **total function** signature over its input domain:

- The only parameter is `uint16_t h`. Every one of the 2^16 = 65536 possible
  `uint16_t` values is a *valid* input; there is no reserved/invalid encoding.
- There are no pointer parameters, so no null-pointer rejection path exists.
- There are no length/size parameters, so no zero-length or oversized-length
  rejection path exists.
- There are no enum parameters, so there is no out-of-range-enum path.
- The return type is `float`, which carries no error sentinel: every one of the
  65536 inputs maps to a returned `float` bit pattern, including NaN and Inf
  patterns, which are *results*, not errors.

Index safety is structural rather than checked: `n = h >> 10` is in `0..=63`
for any `uint16_t`, and `(h & 0x3ff) + m__offset[n]` is at most
`0x3ff + 0x400 == 2047`, always in bounds of the 2048-entry mantissa table.
The C performs no bounds check because none can fail.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| — | `half2float` | *(none — see analysis above)* | *(n/a)* |

**The error-surface table is empty: 0 rows.** There is no rejection path in the
C to differentially test, because the C rejects nothing.

## Boundary conditions covered anyway (Phase C generic boundaries)

Even though no row exists, the generic C-API boundaries that *can* apply to a
`uint16_t`-in / `float`-out function are covered by the exhaustive test in
`tests/differential.rs::exhaustive_all_65536_inputs`, which includes:

| condition | input | test |
|-----------|-------|------|
| minimum value | `h = 0x0000` | covered (exhaustive + `boundary_values`) |
| maximum value | `h = 0xFFFF` | covered (exhaustive + `boundary_values`) |
| one past largest subnormal | `h = 0x0400` | covered |
| largest subnormal | `h = 0x03FF` | covered |
| exponent-field boundaries | every `h` with `h >> 10` = 0,31,32,63 | covered |
| `+Inf` / `-Inf` encodings | `0x7C00`, `0xFC00` | covered |
| every NaN encoding | `0x7C01..0x7FFF`, `0xFC01..0xFFFF` | covered |
| negative zero | `0x8000` | covered |
| the `n == 31` / `n == 63` table quirk (`0x47800000`/`0xc7800000` exponent) | all `h >> 10 == 31` or `63` | covered |
| out-of-range "enum"-like widening across FFI | `uint16_t` has no invalid bit pattern; all 65536 passed | covered |

Because the input domain is only 65536 values wide, the exhaustive test is a
*complete* proof of behavioural equivalence — stronger than any sampled
error-path test could be.

## Completeness

- [x] Every distinct rejection in the C source is enumerated (there are none).
- [x] Generic boundaries (min, max, one-past-range, all special encodings)
      are differentially tested against both `.so`s.
