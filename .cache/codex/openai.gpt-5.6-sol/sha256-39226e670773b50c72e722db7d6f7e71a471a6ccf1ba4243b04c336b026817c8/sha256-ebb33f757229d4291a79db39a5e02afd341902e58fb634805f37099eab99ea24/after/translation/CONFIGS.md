# Configuration Surface

The sole public entry point is the low-level API `normalize(float *dest,
const float *src, int size)`. There are no runtime options, modes, flags,
enums, compile-time features, convenience wrappers, or binaries.

The rows below are the cross-product pruned to distinctions made by the C
loops and branches: `size <= 0` versus positive one/many, `sum > 0` versus
false, finite versus infinite positive sums, and exact/non-exact pointer
aliasing. Partial overlap is split by direction because the positive-sum
write loop proceeds forward and can overwrite unread source elements.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `normalize` | `size == 0`; distinct valid `dest` and `src`; no-op `memset(..., 0)` path | [x] |
| 2 | `normalize` | `size == 0`; exact in-place alias (`dest == src`); no-op alias path | [x] |
| 3 | `normalize` | `size == 1`; finite positive sum; disjoint buffers | [x] |
| 4 | `normalize` | `size == 1`; finite positive sum; exact in-place alias | [x] |
| 5 | `normalize` | `size > 1`; finite positive sum; disjoint buffers | [x] |
| 6 | `normalize` | `size > 1`; finite positive sum; exact in-place alias | [x] |
| 7 | `normalize` | `size > 1`; finite positive sum; partial overlap with `dest < src` | [x] |
| 8 | `normalize` | `size > 1`; finite positive sum; partial overlap with `dest > src` (forward writes can alter unread source) | [x] |
| 9 | `normalize` | `size == 1`; zero sum; disjoint buffers are zero-filled | [x] |
| 10 | `normalize` | `size == 1`; zero sum; exact in-place alias is left byte-identical | [x] |
| 11 | `normalize` | `size > 1`; zero sum; disjoint buffers are zero-filled | [x] |
| 12 | `normalize` | `size > 1`; zero sum; exact in-place alias is left byte-identical | [x] |
| 13 | `normalize` | `size > 1`; zero sum; partial overlap with `dest < src` is zero-filled | [x] |
| 14 | `normalize` | `size > 1`; zero sum; partial overlap with `dest > src` is zero-filled | [x] |
| 15 | `normalize` | positive size; sum becomes NaN; disjoint buffers are zero-filled | [x] |
| 16 | `normalize` | positive size; sum becomes NaN; exact in-place alias is left byte-identical | [x] |
| 17 | `normalize` | positive size; sum becomes NaN; partially overlapping buffers are zero-filled at `dest` | [x] |
| 18 | `normalize` | positive size; sum is positive infinity (infinite input or finite overflow); disjoint buffers | [x] |
| 19 | `normalize` | positive size; sum is positive infinity; exact in-place alias | [x] |
| 20 | `normalize` | `size > 1`; sum is positive infinity; partial overlap with `dest < src` | [x] |
| 21 | `normalize` | `size > 1`; sum is positive infinity; partial overlap with `dest > src` | [x] |
| 22 | `normalize` | positive nonzero subnormal inputs whose squares underflow, yielding zero sum; disjoint buffers | [x] |
| 23 | `normalize` | positive nonzero subnormal inputs whose squares underflow, yielding zero sum; exact in-place alias | [x] |
| 24 | `normalize` | `size < 0`; exact alias (including equal null pointers); loops skipped and no `memset` | [x] |
