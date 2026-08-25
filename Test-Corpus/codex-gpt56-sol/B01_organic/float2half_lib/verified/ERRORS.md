# Error Surface

Mechanical inspection covered every source and public header under `c_src/`.
The only public entry point, `float2half(float)`, accepts a scalar with no
pointer, length, enum, option, or state parameters. The implementation has no
error-return macro, error sentinel, assertion, explicit range check, null
check, or min/max rejection.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|

There are no rejection rows. All 32-bit object representations of the `float`
argument, including infinities, NaNs, signed zero, and subnormal values, are
valid inputs handled through the 512-entry lookup tables.

Phase C applicability check: **complete**. Generic null-pointer, zero/oversized
length, and out-of-range enum cases do not apply to this scalar-only signature.
