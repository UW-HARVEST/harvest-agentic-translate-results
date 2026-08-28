# Configuration Surface

The sole public entry point is:

```c
void gaussian_kernel(float *dest, int size, float radius);
```

There are no compile-time or Cargo feature flags. The source-derived runtime
axes are:

- `size / 2` and the inclusive `-hsize..=hsize` generation loop: no
  generation, one element, exactly `size` elements, or `size + 1` elements.
- The `r < size` normalization loop: zero elements, all generated elements, or
  all except the extra element generated for positive even sizes.
- `rs = 1.6f / radius`: finite, infinite, or NaN. This controls whether at
  least the center sample contributes to `sum`.
- `sum > 0.0f`: normalize or retain generated values unchanged.

For the table, `tiny` means a nonzero finite radius for which `1.6f / radius`
overflows to infinity. Every row is exercised with randomized initial
destination bytes and randomized values within the stated radius class where
that class has more than fixed IEEE-754 special values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `gaussian_kernel` | `size <= -2` (including `INT_MIN`); any radius; no generation or normalization | [x] |
| 2 | `gaussian_kernel` | `size == -1`; finite nonzero radius with finite `rs`; one raw element | [x] |
| 3 | `gaussian_kernel` | `size == -1`; radius `+inf` or `-inf`; one raw element with finite zero `rs` | [x] |
| 4 | `gaussian_kernel` | `size == -1`; radius `+0` or `-0`; one zero element because `rs` is infinite | [x] |
| 5 | `gaussian_kernel` | `size == -1`; tiny nonzero radius; one zero element because `rs` overflows | [x] |
| 6 | `gaussian_kernel` | `size == -1`; NaN radius; one zero element | [x] |
| 7 | `gaussian_kernel` | `size == 0`; finite nonzero radius with finite `rs`; one raw element and zero normalized elements | [x] |
| 8 | `gaussian_kernel` | `size == 0`; radius `+inf` or `-inf`; one raw element and zero normalized elements | [x] |
| 9 | `gaussian_kernel` | `size == 0`; radius `+0` or `-0`; one zero element and zero normalized elements | [x] |
| 10 | `gaussian_kernel` | `size == 0`; tiny nonzero radius; one zero element and zero normalized elements | [x] |
| 11 | `gaussian_kernel` | `size == 0`; NaN radius; one zero element and zero normalized elements | [x] |
| 12 | `gaussian_kernel` | `size == 1`; finite nonzero radius with finite `rs`; one generated and normalized element | [x] |
| 13 | `gaussian_kernel` | `size == 1`; radius `+inf` or `-inf`; one generated and normalized element | [x] |
| 14 | `gaussian_kernel` | `size == 1`; radius `+0` or `-0`; one zero element without normalization | [x] |
| 15 | `gaussian_kernel` | `size == 1`; tiny nonzero radius; one zero element without normalization | [x] |
| 16 | `gaussian_kernel` | `size == 1`; NaN radius; one zero element without normalization | [x] |
| 17 | `gaussian_kernel` | positive odd `size >= 3`; finite nonzero radius with finite `rs`; exactly `size` elements normalized | [x] |
| 18 | `gaussian_kernel` | positive odd `size >= 3`; radius `+inf` or `-inf`; exactly `size` elements normalized | [x] |
| 19 | `gaussian_kernel` | positive odd `size >= 3`; radius `+0` or `-0`; exactly `size` zero elements without normalization | [x] |
| 20 | `gaussian_kernel` | positive odd `size >= 3`; tiny nonzero radius; exactly `size` zero elements without normalization | [x] |
| 21 | `gaussian_kernel` | positive odd `size >= 3`; NaN radius; exactly `size` zero elements without normalization | [x] |
| 22 | `gaussian_kernel` | positive even `size`; finite nonzero radius with finite `rs`; `size + 1` generated, first `size` normalized | [x] |
| 23 | `gaussian_kernel` | positive even `size`; radius `+inf` or `-inf`; `size + 1` generated, first `size` normalized | [x] |
| 24 | `gaussian_kernel` | positive even `size`; radius `+0` or `-0`; `size + 1` zero elements without normalization | [x] |
| 25 | `gaussian_kernel` | positive even `size`; tiny nonzero radius; `size + 1` zero elements without normalization | [x] |
| 26 | `gaussian_kernel` | positive even `size`; NaN radius; `size + 1` zero elements without normalization | [x] |
